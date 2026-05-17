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
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.overlay",
    "Assign values to a dimension based on the extent of an OGR-readable data "
    " source or an OGR SQL query.",
    "https://pdal.org/stages/filters.overlay.html"};

CREATE_STATIC_STAGE(OverlayFilter, s_info)

OverlayFilter::~OverlayFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
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
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
    
    m_rust_stage = pdal_stage_create_overlay(
        m_dimName.c_str(),
        m_datasource.c_str(),
        m_column.c_str());
    
    if (!m_rust_stage)
    {
        std::string err = pdal_last_error();
        if (!err.empty())
            throwError(err);
    }
}

void OverlayFilter::prepared(PointTableRef table)
{
}

void OverlayFilter::ready(PointTableRef table)
{
    if (m_rust_stage)
        pdal_stage_reset(m_rust_stage);
}

void OverlayFilter::spatialReferenceChanged(const SpatialReference& srs)
{
}

bool OverlayFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return true;
}

void OverlayFilter::filter(PointView& view)
{
    if (m_rust_stage)
    {
        rust_view_converter::runInPlace(m_rust_stage, view);
    }
}

} // namespace pdal
