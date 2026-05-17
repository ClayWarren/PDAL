/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
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

#include "CropFilter.hpp"

#include <pdal/PointView.hpp>
#include <pdal/Polygon.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal/private/OGRSpec.hpp>
#include <pdal/private/gdal/GDALUtils.hpp>
#include <pdal/util/Bounds.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include "private/Point.hpp"
#include "private/pnp/GridPnp.hpp"

#include <cstdarg>
#include <sstream>
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.crop",
    "Filter points inside or outside a bounding box or a polygon",
    "https://pdal.org/stages/filters.crop.html"};

CREATE_STATIC_STAGE(CropFilter, s_info)

struct CropArgs
{
    bool m_cropOutside;
    SpatialReference m_assignedSrs;
    std::vector<Bounds> m_bounds;
    std::vector<filter::Point> m_centers;
    double m_distance;
    std::vector<Polygon> m_polys;
    OGRSpec m_ogr;
};

CropFilter::ViewGeom::ViewGeom(const Polygon& poly) : m_poly(poly) {}

CropFilter::ViewGeom::ViewGeom(ViewGeom&& vg)
    : m_poly(vg.m_poly), m_gridPnps(std::move(vg.m_gridPnps))
{
}

std::string CropFilter::getName() const
{
    return s_info.name;
}

CropFilter::CropFilter() : m_args(new CropArgs) {}

CropFilter::~CropFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

void CropFilter::addArgs(ProgramArgs& args)
{
    args.add("outside",
             "Whether we keep points inside or outside of the "
             "bounding region",
             m_args->m_cropOutside);
    args.add("a_srs", "Spatial reference for bounding region",
             m_args->m_assignedSrs);
    args.add("bounds", "Point box for cropped points", m_args->m_bounds);
    args.add("point",
             "Center of circular/spherical crop region.  Use with "
             "'distance'.",
             m_args->m_centers)
        .setErrorText("Invalid point specification.  Must be valid "
                      "GeoJSON/WKT. Ex: \"POINT (1 1)\" or \"POINT (1 1 1)\"");
    args.add("distance", "Crop with this distance from 2D or 3D 'point'",
             m_args->m_distance);
    args.add("polygon", "Bounding polying for cropped points", m_args->m_polys)
        .setErrorText("Invalid polygon specification.  "
                      "Must be valid GeoJSON/WKT");
    args.add("ogr", "OGR filter geometries", m_args->m_ogr);
}

void CropFilter::initialize()
{
    // Set geometry from polygons.
    if (m_args->m_polys.size())
    {
        m_geoms.clear();
        for (Polygon& poly : m_args->m_polys)
        {
            // Throws if invalid.
            poly.valid();
            m_geoms.emplace_back(poly);
        }
    }
    // Add geometry from OGR specification

    for (const Polygon& poly : m_args->m_ogr.getPolygons())
    {
        m_geoms.push_back(poly);
    }

    m_boxes.clear();
    for (auto& bound : m_args->m_bounds)
        m_boxes.push_back(bound);

    m_distance2 = m_args->m_distance * m_args->m_distance;

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    std::vector<pdal_box3d_t> bounds;
    for (auto const& b : m_boxes)
    {
        bounds.push_back({b.to3d().minx, b.to3d().miny, b.to3d().minz, b.to3d().maxx, b.to3d().maxy, b.to3d().maxz});
    }

    std::vector<std::string> polys_wkt;
    for (auto const& g : m_geoms)
        polys_wkt.push_back(g.m_poly.wkt());

    std::vector<const char*> polys;
    for (auto const& s : polys_wkt)
        polys.push_back(s.c_str());

    std::vector<pdal_point3d_t> centers;
    for (auto const& c : m_args->m_centers)
        centers.push_back({c.x(), c.y(), c.z()});

    m_rust_stage = pdal_stage_create_crop(
        m_args->m_cropOutside,
        bounds.data(),
        bounds.size(),
        polys.data(),
        polys.size(),
        centers.data(),
        centers.size(),
        m_args->m_distance);
    
    if (!m_rust_stage)
    {
        std::string err = pdal_last_error();
        if (!err.empty())
            throwError(err);
    }
}

void CropFilter::ready(PointTableRef table)
{
    if (m_rust_stage)
        pdal_stage_reset(m_rust_stage);
}

bool CropFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return false;
}

void CropFilter::spatialReferenceChanged(const SpatialReference& srs)
{
}

void CropFilter::transform(const SpatialReference& srs)
{
}

PointViewSet CropFilter::run(PointViewPtr view)
{
    PointViewSet viewSet;
    if (m_rust_stage)
    {
        viewSet.insert(rust_view_converter::runSingle(m_rust_stage, view));
    }
    return viewSet;
}

bool CropFilter::crop(const PointRef& point, const BOX3D& box) { return false; }
bool CropFilter::crop(const PointRef& point, const BOX2D& box) { return false; }
void CropFilter::crop(const Bounds& box, PointView& input, PointView& output) {}
void CropFilter::crop(const BOX3D& box, PointView& input, PointView& output) {}
void CropFilter::crop(const BOX2D& box, PointView& input, PointView& output) {}
bool CropFilter::crop(const PointRef& point, GridPnp& g) { return false; }
void CropFilter::crop(const ViewGeom& g, PointView& input, PointView& output) {}
bool CropFilter::crop(const PointRef& point, const filter::Point& center) { return false; }
void CropFilter::crop(const filter::Point& center, PointView& input, PointView& output) {}

} // namespace pdal
