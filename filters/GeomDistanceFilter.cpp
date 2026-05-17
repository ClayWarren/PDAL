/******************************************************************************
 * Copyright (c) 2022, Howard Butler (info@hobu.co)
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
 *     * Neither the name of Hobu, Inc. or Flaxen Geo Consulting nor the
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

#include "GeomDistanceFilter.hpp"

#include <string>
#include <vector>

#include <pdal/Geometry.hpp>
#include <pdal/Polygon.hpp>
#include <pdal/private/OGRSpec.hpp>
#include <pdal/private/gdal/GDALUtils.hpp>
#include <pdal/util/Bounds.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <nlohmann/json.hpp>
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.geomdistance",
    "Compute the distance for points to a given geometry",
    "https://pdal.org/stages/filters.geomdistance.html"};

CREATE_STATIC_STAGE(GeomDistanceFilter, s_info)

struct GeomDistanceArgs
{
    Dimension::Id m_dim;
    std::string m_dimName;
    pdal::Geometry m_geometry;
    bool m_doRingMode;
    OGRSpec m_ogr;
};

GeomDistanceFilter::GeomDistanceFilter() : m_args(new GeomDistanceArgs) {}

GeomDistanceFilter::~GeomDistanceFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

std::string GeomDistanceFilter::getName() const
{
    return s_info.name;
}

void GeomDistanceFilter::addDimensions(PointLayoutPtr layout)
{
    m_args->m_dim =
        layout->registerOrAssignDim(m_args->m_dimName, Dimension::Type::Double);
}

void GeomDistanceFilter::initialize()
{
    gdal::registerDrivers();
}

void GeomDistanceFilter::addArgs(ProgramArgs& args)
{
    args.add("geometry", "Geometries to test", m_args->m_geometry)
        .setErrorText("Invalid polygon specification.  "
                      "Must be valid GeoJSON/WKT");
    args.add("dimension", "Dimension to create to place distance values",
             m_args->m_dimName, "distance");
    args.add("ring", "Compare edges (demote polygons to linearrings)",
             m_args->m_doRingMode, false);
    args.add("ogr", "OGR filter geometries", m_args->m_ogr);
}

void GeomDistanceFilter::prepared(PointTableRef table)
{
}

void GeomDistanceFilter::ready(PointTableRef table)
{
    if (!m_args->m_ogr.empty())
        m_args->m_geometry = m_args->m_ogr.getPolygons()[0];

    if (m_args->m_doRingMode)
        m_args->m_geometry = m_args->m_geometry.getRing();

    if (!m_args->m_geometry.getOGRHandle())
        throwError("Candidate polygon in filters.geomdistance was NULL!");

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
    
    m_rust_stage = pdal_stage_create_geomdistance(
        m_args->m_geometry.wkt().c_str(),
        m_args->m_dimName.c_str());
    
    if (!m_rust_stage)
    {
        std::string err = pdal_last_error();
        if (!err.empty())
            throwError(err);
    }
}

void GeomDistanceFilter::filter(PointView& view)
{
    if (m_rust_stage)
    {
        rust_view_converter::runInPlace(m_rust_stage, view);
    }
}

bool GeomDistanceFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return false;
}

} // namespace pdal
