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
#include "private/georeference/LocalCartesian.hpp"
#include "private/georeference/Trajectory.hpp"
#include "private/georeference/Utils.hpp"
#include <nlohmann/json.hpp>

#include "GeoreferenceFilter.hpp"
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

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
    Eigen::Affine3d m_scan2imu;
    bool m_ned;
    EIGEN_MAKE_ALIGNED_OPERATOR_NEW
    Config()
        : m_trajectoryFile(""), m_coordinateSystem("NED"), m_timeOffset(0.0),
          m_reverse(false), m_transformBeam(false), m_trajectory(nullptr)
    {
    }
    void init()
    {
        m_trajectory.reset(new georeference::Trajectory(m_trajectoryFile,
                                                        m_trajectoryOptions));
        Eigen::Matrix4d m;
        m << m_matrix[0], m_matrix[1], m_matrix[2], m_matrix[3], m_matrix[4],
            m_matrix[5], m_matrix[6], m_matrix[7], m_matrix[8], m_matrix[9],
            m_matrix[10], m_matrix[11], m_matrix[12], m_matrix[13],
            m_matrix[14], m_matrix[15];
        m_scan2imu.matrix() = m;
        std::string s = Utils::toupper(m_coordinateSystem);
        if (s == "NED")
            m_ned = true;
        else if (s == "ENU")
            m_ned = false;
        else
            throw pdal_error("Local Tangent Plane coordinate system " +
                             m_coordinateSystem + " is not allowed.");
    }
};

GeoreferenceFilter::GeoreferenceFilter()
    : Filter(), Streamable(), m_config(new Config),
      m_localCartesian(new georeference::LocalCartesian(0.0, 0.0, 0.0))

{
}

GeoreferenceFilter::~GeoreferenceFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

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

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    m_rust_stage = pdal_stage_create_georeference(srs.getWKT().c_str());
    if (!m_rust_stage)
    {
        std::string err = pdal_last_error();
        if (!err.empty())
            throwError(err);
    }
}

void GeoreferenceFilter::ready(PointTableRef table)
{
    if (m_rust_stage)
        pdal_stage_reset(m_rust_stage);
}

void GeoreferenceFilter::prepared(PointTableRef table)
{
}

bool GeoreferenceFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        pdal_point_view_set_spatial_reference((pdal_point_view_t*)point.view(), point.view()->spatialReference().getWKT().c_str());
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return false;
}

void GeoreferenceFilter::filter(PointView& view)
{
    if (m_rust_stage)
    {
        pdal_point_view_set_spatial_reference((pdal_point_view_t*)&view, view.spatialReference().getWKT().c_str());
        rust_view_converter::runInPlace(m_rust_stage, view);
    }
}

} // namespace pdal
