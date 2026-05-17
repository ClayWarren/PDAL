/******************************************************************************
 * Copyright (c) 2016, Howard Butler, hobu.inc@gmail.com
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

#include "ColorinterpFilter.hpp"

#include <pdal/PointView.hpp>
#include <pdal/private/gdal/GDALUtils.hpp>
#include <pdal/private/gdal/Raster.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal/util/Utils.hpp>

#include <algorithm>
#include <array>
#include <cmath>

#include <cpl_vsi.h>

#include "ColorInterpRamps.hpp"
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

namespace pdal
{

static std::vector<std::string> ramps = {
    "awesome_green", "black_orange",  "blue_hue",   "blue_red",
    "heat_map",      "pestel_shades", "blue_orange"};

static StaticPluginInfo const s_info{
    "filters.colorinterp", "Assigns RGB colors based on a dimension and a ramp",
    "https://pdal.org/stages/filters.colorinterp.html"};

CREATE_STATIC_STAGE(ColorinterpFilter, s_info)

std::string ColorinterpFilter::getName() const
{
    return s_info.name;
}

ColorinterpFilter::~ColorinterpFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

// The VSIFILE* that VSIFileFromMemBuffer creates in this
// macro is never cleaned up. We're opening seven PNGs in the
// ColorInterpRamps-ramps.hpp header. We always open them so they're available.
//
// GDAL forces to keep track of the return value, and its being ignored here,
// To avoid the warning message:
// warning: ignoring return value of 'VSILFILE* VSIFileFromMemBuffer(....)'
//          declared with attribute warn_unused_result [-Wunused-result]
// Using a tmp variable
#define GETRAMP(name)                                                          \
    if (pdal::Utils::iequals(#name, rampFilename))                             \
    {                                                                          \
        unsigned char* location(name);                                         \
        int size(0);                                                           \
        location = name;                                                       \
        size = sizeof(name);                                                   \
        rampFilename = "/vsimem/" + std::string(#name) + ".png";               \
        auto tmp(VSIFileFromMemBuffer(rampFilename.c_str(), location, size,    \
                                      false));                                 \
    }
//

std::shared_ptr<gdal::Raster> openRamp(std::string& rampFilename)
{
    GETRAMP(awesome_green);
    GETRAMP(black_orange);
    GETRAMP(blue_hue);
    GETRAMP(blue_red);
    GETRAMP(heat_map);
    GETRAMP(pestel_shades);
    GETRAMP(blue_orange);

    std::shared_ptr<gdal::Raster> output(
        new gdal::Raster(rampFilename.c_str()));
    return output;
}

void ColorinterpFilter::addArgs(ProgramArgs& args)
{
    args.add("dimension", "Dimension to interpolate", m_interpDimString, "Z");
    args.add("minimum", "Minimum value to use for scaling", m_min,
             std::numeric_limits<double>::quiet_NaN());
    args.add("maximum", "Maximum value to use for scaling", m_max,
             std::numeric_limits<double>::quiet_NaN());
    args.add("clamp",
             "Clamp and color values outside the range [minimum, maximum]",
             m_clamp, false);
    args.add("ramp", "GDAL-readable color ramp image to use", m_colorramp,
             "pestel_shades");
    args.add("invert", "Invert the ramp direction", m_invertRamp, false);
    args.add("mad",
             "Use Median Absolute Deviation to compute ramp bounds "
             "in combination with 'k' ",
             m_useMAD, false);
    args.add("mad_multiplier", "MAD threshold multiplier", m_madMultiplier,
             1.4862);
    args.add("k", "Number of deviations to compute minimum/maximum ",
             m_stdDevThreshold, 0.0);
}

void ColorinterpFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDims(
        {Dimension::Id::Red, Dimension::Id::Green, Dimension::Id::Blue});
}

void ColorinterpFilter::prepared(PointTableRef table)
{
}

/**
  Read the band data into local vectors.

  \param table  Point table.
*/
void ColorinterpFilter::ready(PointTableRef table)
{
    gdal::registerDrivers();

    // Setup the ramp filename if it's a built-in one
    std::string rampFilename = m_colorramp;
    openRamp(rampFilename);

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
    
    m_rust_stage = pdal_stage_create_colorinterp(
        m_interpDimString.c_str(),
        rampFilename.c_str(),
        m_min,
        m_max,
        m_clamp,
        m_invertRamp);
    
    if (!m_rust_stage)
    {
        std::string err = pdal_last_error();
        if (!err.empty())
            throwError(err);
    }
}

void ColorinterpFilter::filter(PointView& view)
{
    if (m_rust_stage)
    {
        rust_view_converter::runInPlace(m_rust_stage, view);
    }
}

bool ColorinterpFilter::pipelineStreamable() const
{
    if (std::isnan(m_min) || std::isnan(m_max))
        return false;
    return Streamable::pipelineStreamable();
}

bool ColorinterpFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return false;
}

} // namespace pdal
