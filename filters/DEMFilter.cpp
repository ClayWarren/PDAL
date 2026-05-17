/******************************************************************************
 * Copyright (c) 2017, Howard Butler (info@hobu.co)
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

#include "DEMFilter.hpp"

#include <string>
#include <vector>

#include "private/DimRange.hpp"
#include <pdal/private/gdal/Raster.hpp>
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.dem", "Filter points about an elevation surface",
    "https://pdal.org/stages/filters.dem.html"};

CREATE_STATIC_STAGE(DEMFilter, s_info)

struct DEMArgs
{
    Dimension::Id m_dim;
    DimRange m_range;
    std::string m_raster;
    int32_t m_band;
};

DEMFilter::DEMFilter() : m_args(new DEMArgs) {}

DEMFilter::~DEMFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

std::string DEMFilter::getName() const
{
    return s_info.name;
}

void DEMFilter::addDimensions(PointLayoutPtr layout) {}

void DEMFilter::addArgs(ProgramArgs& args)
{
    args.add("limits", "Dimension limits for filtering", m_args->m_range)
        .setPositional();
    args.add("raster", "GDAL-readable raster to use for DEM", m_args->m_raster)
        .setPositional();
    args.add("band", "Band number to filter (count from 1)", m_args->m_band, 1);
}

void DEMFilter::initialize()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
    
    m_rust_stage = pdal_stage_create_dem(
        m_args->m_range.m_name.c_str(),
        m_args->m_raster.c_str(),
        m_args->m_band,
        m_args->m_range.m_lower_bound,
        m_args->m_range.m_upper_bound);
    
    if (!m_rust_stage)
    {
        std::string err = pdal_last_error();
        if (!err.empty())
            throwError(err);
    }
}

void DEMFilter::ready(PointTableRef table)
{
    if (m_rust_stage)
        pdal_stage_reset(m_rust_stage);
}

void DEMFilter::prepared(PointTableRef table)
{
}

bool DEMFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return false;
}

PointViewSet DEMFilter::run(PointViewPtr inView)
{
    PointViewSet viewSet;
    if (m_rust_stage)
    {
        viewSet.insert(rust_view_converter::runSingle(m_rust_stage, inView));
    }
    return viewSet;
}

} // namespace pdal
