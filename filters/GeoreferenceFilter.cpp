/******************************************************************************
 * Copyright (c) 2023, Guilhem Villemin (guilhem.villemin@altametris.com)
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in
 *       the documentation and/or other materials provided
 *       with the distribution.
 *     * Neither the name of Hobu, Inc. or Flaxen Consulting LLC nor the
 *       names of its contributors may be used to endorse or promote
 *       products derived from this software without specific prior
 *       written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include "TransformationFilter.hpp"
#include "private/georeference/Trajectory.hpp"
#include <nlohmann/json.hpp>
#include <pdal/util/Utils.hpp>

#include "GeoreferenceFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal_capi.h>

namespace pdal
{
static PluginInfo const s_info{
    "filters.georeference", "Georeferencing filter",
    "https://pdal.org/stages/filters.georeference.html"};

CREATE_STATIC_STAGE(GeoreferenceFilter, s_info)

using DimId = Dimension::Id;

struct GeoreferenceFilter::Config
{
public:
    TransformationFilter::Transform m_matrix;
    std::string m_trajectoryFile;
    NL::json m_trajectoryOptions;
    std::string m_coordinateSystem;
    double m_timeOffset;
    bool m_reverse;
    bool m_transformBeam;
    std::unique_ptr<georeference::Trajectory> m_trajectory;
    bool m_ned;
    Config()
        : m_trajectoryFile(""), m_coordinateSystem("NED"), m_timeOffset(0.0),
          m_reverse(false), m_transformBeam(false), m_trajectory(nullptr),
          m_ned(false)
    {
    }
    void init()
    {
        m_trajectory.reset(new georeference::Trajectory(m_trajectoryFile,
                                                        m_trajectoryOptions));
        char* error =
            pdal_georeference_validate_coordinate_system(m_coordinateSystem.c_str());
        if (error)
        {
            std::string message(error);
            pdal_string_free(error);
            throw pdal_error(message);
        }
        m_ned = Utils::toupper(m_coordinateSystem) == "NED";
    }
};

GeoreferenceFilter::GeoreferenceFilter()
    : Filter(), Streamable(), m_config(new Config)
{
}
GeoreferenceFilter::~GeoreferenceFilter() {}

std::string GeoreferenceFilter::getName() const
{
    return s_info.name;
}

void GeoreferenceFilter::addArgs(ProgramArgs& args)
{
    args.add("trajectory_file", "Path to trajectory file",
             m_config->m_trajectoryFile)
        .setPositional();
    args.add("trajectory_options", "Trajectory reader option",
             m_config->m_trajectoryOptions);
    args.add("scan2imu", "Transformation from scanner to imu",
             m_config->m_matrix)
        .setPositional();
    args.add("reverse", "reverse georeferencing", m_config->m_reverse,
             m_config->m_reverse);
    args.add("time_offset",
             "time offset between trajectory and scanner timestamps",
             m_config->m_timeOffset, m_config->m_timeOffset);
    args.add("coordinate_system", "scan2imu coordinate system",
             m_config->m_coordinateSystem, m_config->m_coordinateSystem);
    args.add("transform_beam",
             "Transform BeamOrigin and BeamDirection dimensions",
             m_config->m_transformBeam, m_config->m_transformBeam);
}

void GeoreferenceFilter::initialize()
{
    m_config->init();
    SpatialReference srs("EPSG:4978");
    setSpatialReference(srs);
}

void GeoreferenceFilter::prepared(PointTableRef table)
{
    pdal_point_layout_t* rustLayout = pdal_point_layout_create();
    for (auto dim : table.layout()->dims())
    {
        pdal_point_layout_register_dim(
            rustLayout, table.layout()->dimName(dim).c_str(),
            rust_view_converter::typeId(table.layout()->dimType(dim)));
    }

    char* error = pdal_georeference_validate_transform_beam(
        rustLayout, m_config->m_transformBeam);
    pdal_point_layout_destroy(rustLayout);
    if (error)
    {
        std::string message(error);
        pdal_string_free(error);
        throwError(message);
    }
}

bool GeoreferenceFilter::processOne(PointRef& point)
{
    // The full per-point georeferencing (trajectory interpolation, the
    // rotation/scan2imu composition, the local-cartesian frame, and the
    // forward/reverse + NED/ENU + beam handling) runs in Rust through
    // pdal_georeference_process_point. This C++ method just marshals the
    // point's dimensions across the C ABI.
    double scan2imu[16];
    for (size_t i = 0; i < 16; ++i)
        scan2imu[i] = m_config->m_matrix[i];

    double x = point.getFieldAs<double>(DimId::X);
    double y = point.getFieldAs<double>(DimId::Y);
    double z = point.getFieldAs<double>(DimId::Z);
    double gpsTime = point.getFieldAs<double>(Dimension::Id::GpsTime);

    double box = 0, boy = 0, boz = 0, bdx = 0, bdy = 0, bdz = 0;
    if (m_config->m_transformBeam)
    {
        box = point.getFieldAs<double>(DimId::BeamOriginX);
        boy = point.getFieldAs<double>(DimId::BeamOriginY);
        boz = point.getFieldAs<double>(DimId::BeamOriginZ);
        bdx = point.getFieldAs<double>(DimId::BeamDirectionX);
        bdy = point.getFieldAs<double>(DimId::BeamDirectionY);
        bdz = point.getFieldAs<double>(DimId::BeamDirectionZ);
    }

    if (!pdal_georeference_process_point(
            m_config->m_trajectory->handle(), scan2imu, m_config->m_reverse,
            m_config->m_ned, m_config->m_transformBeam, m_config->m_timeOffset,
            gpsTime, &x, &y, &z, &box, &boy, &boz, &bdx, &bdy, &bdz))
        return false;

    point.setField(DimId::X, x);
    point.setField(DimId::Y, y);
    point.setField(DimId::Z, z);
    if (m_config->m_transformBeam)
    {
        point.setField(DimId::BeamOriginX, box);
        point.setField(DimId::BeamOriginY, boy);
        point.setField(DimId::BeamOriginZ, boz);
        point.setField(DimId::BeamDirectionX, bdx);
        point.setField(DimId::BeamDirectionY, bdy);
        point.setField(DimId::BeamDirectionZ, bdz);
    }
    return true;
}

void GeoreferenceFilter::filter(PointView& view)
{
    PointRef point(view, 0);
    for (PointId idx = 0; idx < view.size(); ++idx)
    {
        point.setPointId(idx);
        processOne(point);
    }
    view.invalidateProducts();
}

} // namespace pdal
