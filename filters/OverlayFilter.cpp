/******************************************************************************
 * Copyright (c) 2017, Hobu Inc., info@hobu.co
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

#include "OverlayFilter.hpp"

#include <thread>
#include <vector>

#include <ogr_api.h>

#include <pdal/Polygon.hpp>
#include <pdal/private/gdal/GDALUtils.hpp>
#include <pdal/private/gdal/SpatialRef.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <pdal/private/RustViewConverter.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.overlay",
    "Assign values to a dimension based on the extent of an OGR-readable data "
    " source or an OGR SQL query.",
    "https://pdal.org/stages/filters.overlay.html"};

CREATE_STATIC_STAGE(OverlayFilter, s_info)

OverlayFilter::OverlayFilter()
    : m_ds(nullptr), m_lyr(nullptr), m_rustStage(nullptr)
{
}

OverlayFilter::~OverlayFilter()
{
    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
}

void OverlayFilter::addArgs(ProgramArgs& args)
{
    args.add("dimension", "Dimension on which to filter", m_dimName)
        .setPositional();
    args.add("datasource",
             "OGR-readable datasource for Polygon or "
             "Multipolygon data",
             m_datasource)
        .setPositional();
    args.add("column",
             "OGR datasource column from which to "
             "read the attribute.",
             m_column);
    args.add("query",
             "OGR SQL query to execute on the "
             "datasource to fetch geometry and attributes",
             m_query);
    args.add("layer", "Datasource layer to use", m_layer);
    args.addSynonym("layer", "lyr_name");
    args.add("bounds",
             "Bounds to limit query using with OGR_L_SetSpatialFilter",
             m_bounds);
    args.add("threads", "Number of threads used to run this filter", m_threads,
             1);
}

void OverlayFilter::initialize()
{
    gdal::registerDrivers();
}

void OverlayFilter::prepared(PointTableRef table)
{
    m_layout = table.layout();
    m_dim = table.layout()->findDim(m_dimName);
    if (m_dim == Dimension::Id::Unknown)
        throwError("Dimension '" + m_dimName + "' not found.");
    if (m_threads < 1)
        throwError("Number of threads should be positive.");
}

void OverlayFilter::ready(PointTableRef table)
{
    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
    m_rustStage = pdal_stage_create_overlay(
        m_dimName.c_str(), m_datasource.c_str(),
        m_column.empty() ? nullptr : m_column.c_str());
    if (!m_rustStage)
    {
        const char* message = pdal_last_error();
        if (message && message[0])
            throwError(std::string("filters.overlay: ") + message);
        throwError("Failed to create Rust overlay stage.");
    }

    m_ds = OGRDSPtr(OGROpen(m_datasource.c_str(), 0, nullptr),
                    [](OGRDSPtr::element_type* p)
                    {
                        if (p)
                            ::OGR_DS_Destroy(p);
                    });
    if (!m_ds)
        throwError("Unable to open data source '" + m_datasource + "'");

    if (m_layer.size())
        m_lyr = OGR_DS_GetLayerByName(m_ds.get(), m_layer.c_str());
    else if (m_query.size())
        m_lyr =
            OGR_DS_ExecuteSQL(m_ds.get(), m_query.c_str(), nullptr, nullptr);
    else
        m_lyr = OGR_DS_GetLayer(m_ds.get(), 0);

    if (!m_lyr)
        throwError("Unable to select layer '" + m_layer + "'");

    if (!m_bounds.empty())
    {
        pdal::Polygon g(m_bounds.toWKT());
        OGR_L_SetSpatialFilter(m_lyr, g.getOGRHandle());
    }

    auto featureDeleter = [](OGRFeaturePtr::element_type* p)
    {
        if (p)
            ::OGR_F_Destroy(p);
    };
    OGRFeaturePtr feature =
        OGRFeaturePtr(OGR_L_GetNextFeature(m_lyr), featureDeleter);

    int field_index(1); // default to first column if nothing was set
    if (m_column.size())
    {
        field_index = OGR_F_GetFieldIndex(feature.get(), m_column.c_str());
        if (field_index == -1)
            throwError("No column name '" + m_column + "' was found.");
    }

    gdal::SpatialRef sref;
    sref.setFromLayer(m_lyr);
    SpatialReference layerSrs(sref.wkt());

    do
    {
        OGRGeometryH geom = OGR_F_GetGeometryRef(feature.get());
        int32_t fieldVal = OGR_F_GetFieldAsInteger(feature.get(), field_index);

        m_polygons.push_back({Polygon(geom, layerSrs), fieldVal});

        feature = OGRFeaturePtr(OGR_L_GetNextFeature(m_lyr), featureDeleter);
    } while (feature);

    // Initialise m_grids, otherwise this will lead to a race condition when
    // using threading.
    for (const auto& poly : m_polygons)
    {
        poly.geom.initGrids();
    }
}

void OverlayFilter::spatialReferenceChanged(const SpatialReference& srs)
{
    if (srs.empty())
        return;
    for (auto& poly : m_polygons)
    {
        auto ok = poly.geom.transform(srs);
        if (!ok)
            throwError(ok.what());
    }
}

bool OverlayFilter::processOne(PointRef& point)
{
    pdal_point_view_t* rustPoint =
        rust_view_converter::toRustPoint(point, m_layout);
    const bool keep = pdal_stage_process_one_at(m_rustStage, rustPoint, 0);
    if (keep)
        rust_view_converter::fromRustPoint(rustPoint, 0, point);
    pdal_point_view_destroy(rustPoint);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError("Rust overlay filter failed.");
    return keep;
}

void OverlayFilter::filter(PointView& view)
{
    rust_view_converter::runInPlace(m_rustStage, view);
}

} // namespace pdal
