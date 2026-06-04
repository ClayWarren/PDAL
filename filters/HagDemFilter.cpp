/******************************************************************************
 * Copyright (c) 2020, Julian Fell (hi@jtfell.com)
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

#include "HagDemFilter.hpp"

#include <algorithm>
#include <pdal/private/gdal/Raster.hpp>
#include <pdal_capi.h>

#include <pdal/private/RustViewConverter.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.hag_dem", "Computes height above ground using a DEM raster.",
    "https://pdal.org/stages/filters.hag_dem.html"};

CREATE_STATIC_STAGE(HagDemFilter, s_info)

std::string HagDemFilter::getName() const
{
    return s_info.name;
}

HagDemFilter::HagDemFilter() : m_rustStage(nullptr) {}

HagDemFilter::~HagDemFilter()
{
    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
}

void HagDemFilter::addArgs(ProgramArgs& args)
{
    args.add("raster",
             "GDAL-readable raster to use for DEM (uses band 1, "
             "starting from 1)",
             m_rasterName)
        .setPositional();
    args.add("band", "Band number to filter (count from 1)", m_band, 1);
    args.add("zero_ground",
             "If true, set HAG of ground-classified points "
             "to 0 rather than comparing Z value to raster DEM",
             m_zeroGround, true);
    args.add("min_clamp", "Minimum HAG value", m_minClamp,
             (std::numeric_limits<double>::min)());
    args.add("max_clamp", "Maximum HAG value", m_maxClamp,
             (std::numeric_limits<double>::max)());
    args.add("nodata_hag", "HAG value to use for nodata pixels", m_noDataHeight,
             0.0);
    args.add("class", "Class to use for ground points. [Default: 2]", m_class,
             ClassLabel::Ground);
}

void HagDemFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Dimension::Id::HeightAboveGround);
    m_layout = layout;
}

void HagDemFilter::ready(PointTableRef table)
{
    m_raster.reset(new gdal::Raster(m_rasterName));

    log()->get(LogLevel::Debug)
        << "Nodata HAG override was set to " << m_noDataHeight << '\n';

    if (m_zeroGround)
        log()->get(LogLevel::Debug)
            << "Setting ground-classified points to 0 HAG" << '\n';

    if (m_minClamp != (std::numeric_limits<double>::min)())
        log()->get(LogLevel::Debug)
            << "min_clamp set to " << m_minClamp << '\n';

    if (m_maxClamp != (std::numeric_limits<double>::max)())
        log()->get(LogLevel::Debug)
            << "max_clamp set to " << m_maxClamp << '\n';

    gdal::GDALError response = m_raster->open();
    if (response == gdal::GDALError::NotOpen)
    {
        log()->get(LogLevel::Error)
            << "Unable to open raster " << m_rasterName << '\n';
        throwError(m_raster->errorMsg());
    }

    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
    m_rustStage = pdal_stage_create_hag_dem(
        m_rasterName.c_str(), m_band, m_zeroGround, m_minClamp, m_maxClamp,
        m_noDataHeight, m_class);
    if (!m_rustStage)
        rust_view_converter::throwLastError(
            "Unable to create Rust HAG DEM stage.");
}

void HagDemFilter::prepared(PointTableRef table)
{
    if (m_band <= 0)
        throwError("Band must be greater than 0");
}

void HagDemFilter::filter(PointView& view)
{
    rust_view_converter::runInPlace(m_rustStage, view);
}

bool HagDemFilter::processOne(PointRef& point)
{
    pdal_point_view_t* rustView =
        rust_view_converter::toRustPoint(point, m_layout);
    pdal_stage_process_one_at(m_rustStage, rustView, 0);
    rust_view_converter::fromRustPoint(rustView, 0, point);
    pdal_point_view_destroy(rustView);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError("Rust HAG DEM stage failed.");
    return true;
}

} // namespace pdal
