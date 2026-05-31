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
#include <pdal/private/RustViewConverter.hpp>
#include <pdal/private/gdal/GDALUtils.hpp>
#include <pdal/private/gdal/Raster.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <vector>

#include <cpl_vsi.h>

namespace pdal
{

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
    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
}

std::shared_ptr<gdal::Raster> openRamp(std::string& rampFilename)
{
    const uint8_t* data = nullptr;
    uint64_t size = 0;
    if (pdal_colorinterp_default_ramp(rampFilename.c_str(), &data, &size))
    {
        rampFilename = "/vsimem/" + rampFilename + ".png";
        auto tmp = VSIFileFromMemBuffer(
            rampFilename.c_str(),
            const_cast<GByte*>(reinterpret_cast<const GByte*>(data)),
            static_cast<vsi_l_offset>(size), false);
        (void)tmp;
    }

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
    PointLayoutPtr layout(table.layout());
    m_layout = layout;

    pdal_point_layout_t* rustLayout = pdal_point_layout_create();
    for (auto dim : layout->dims())
    {
        pdal_point_layout_register_dim(
            rustLayout, layout->dimName(dim).c_str(),
            rust_view_converter::typeId(layout->dimType(dim)));
    }

    char* error = pdal_colorinterp_validate_prepared(
        rustLayout, m_interpDimString.c_str(), m_min, m_max);
    pdal_point_layout_destroy(rustLayout);
    if (error)
    {
        std::string message(error);
        pdal_string_free(error);
        throwError(message);
    }

    m_interpDim = layout->findDim(m_interpDimString);

    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
    m_rustStage = pdal_stage_create_colorinterp(
        Dimension::name(m_interpDim).c_str(), m_colorramp.c_str(), m_min, m_max,
        m_clamp, m_invertRamp);
    if (!m_rustStage)
        rust_view_converter::throwLastError(
            "Unable to create Rust colorinterp stage.");
    pdal_stage_destroy(m_rustStage);
    m_rustStage = nullptr;
}

/**
  Read the band data into local vectors.

  \param table  Point table.
*/
void ColorinterpFilter::ready(PointTableRef table)
{
    gdal::registerDrivers();

    m_raster = openRamp(m_colorramp);
    gdal::GDALError err = m_raster->open();
    if (err != gdal::GDALError::None && err != gdal::GDALError::NoTransform)
        throwError(m_raster->errorMsg());

    log()->get(LogLevel::Debug)
        << getName() << " raster connection: " << m_raster->filename() << '\n';

    m_raster->readBand(m_redBand, 1);
    m_raster->readBand(m_greenBand, 2);
    m_raster->readBand(m_blueBand, 3);
}

void ColorinterpFilter::filter(PointView& view)
{
    double median(0.0);
    double mad(0.0);

    // If the user set a 'k' value, we use that
    // defaulting to using a computed stddev if we
    // were not told to use MAD.
    if (m_stdDevThreshold != 0.0)
    {
        std::vector<double> values(view.size());

        stats::Summary summary(Dimension::name(m_interpDim),
                               stats::Summary::NoEnum, false);
        for (PointId idx = 0; idx < view.size(); ++idx)
        {
            double v = view.getFieldAs<double>(m_interpDim, idx);
            summary.insert(v);
            values[idx] = v;
        }

        auto compute_median = [](std::vector<double> vals)
        {
            std::nth_element(vals.begin(), vals.begin() + vals.size() / 2,
                             vals.end());

            return *(vals.begin() + vals.size() / 2);
        };

        median = compute_median(values);
        if (m_useMAD)
        {
            std::transform(values.begin(), values.end(), values.begin(),
                           [median](double v)
                           { return std::fabs(v - median); });
            mad = compute_median(values);

            double threshold = mad * m_madMultiplier * m_stdDevThreshold;
            m_min = median - threshold;
            m_max = median + threshold;

            log()->get(LogLevel::Debug) << getName() << " mad " << mad << '\n';
            log()->get(LogLevel::Debug)
                << getName() << " median " << median << '\n';
            log()->get(LogLevel::Debug)
                << getName() << " minimum " << m_min << '\n';
            log()->get(LogLevel::Debug)
                << getName() << " maximum " << m_max << '\n';
        }
        else
        {
            double threshold = (m_stdDevThreshold * summary.sampleStddev());
            m_min = median - threshold;
            m_max = median + threshold;

            log()->get(LogLevel::Debug)
                << getName() << " stddev threshold " << threshold << '\n';
            log()->get(LogLevel::Debug)
                << getName() << " median " << median << '\n';
            log()->get(LogLevel::Debug)
                << getName() << " minimum " << m_min << '\n';
            log()->get(LogLevel::Debug)
                << getName() << " maximum " << m_max << '\n';
        }
    }

    // If the user didn't set min/max values and hadn't set a stddev, we
    // compute them.
    else if (std::isnan(m_min) || std::isnan(m_max))
    {
        stats::Summary summary(Dimension::name(m_interpDim),
                               stats::Summary::NoEnum, false);
        for (PointId idx = 0; idx < view.size(); ++idx)
        {
            double v = view.getFieldAs<double>(m_interpDim, idx);
            summary.insert(v);
        }

        if (std::isnan(m_min))
            m_min = summary.minimum();
        if (std::isnan(m_max))
            m_max = summary.maximum();
    }

    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
    m_rustStage = pdal_stage_create_colorinterp(
        Dimension::name(m_interpDim).c_str(), m_colorramp.c_str(), m_min, m_max,
        m_clamp, m_invertRamp);
    if (!m_rustStage)
        rust_view_converter::throwLastError(
            "Unable to create Rust colorinterp stage.");
    rust_view_converter::runInPlace(m_rustStage, view);
}

bool ColorinterpFilter::pipelineStreamable() const
{
    if (!pdal_colorinterp_pipeline_streamable(m_min, m_max))
        return false;
    return Streamable::pipelineStreamable();
}

bool ColorinterpFilter::processOne(PointRef& point)
{
    if (!m_rustStage)
    {
        m_rustStage = pdal_stage_create_colorinterp(
            Dimension::name(m_interpDim).c_str(), m_colorramp.c_str(), m_min,
            m_max, m_clamp, m_invertRamp);
        if (!m_rustStage)
            rust_view_converter::throwLastError(
                "Unable to create Rust colorinterp stage.");
    }

    pdal_point_view_t* rustView =
        rust_view_converter::toRustPoint(point, m_layout);
    pdal_stage_process_one_at(m_rustStage, rustView, 0);
    rust_view_converter::fromRustPoint(rustView, 0, point);
    pdal_point_view_destroy(rustView);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError("Rust colorinterp stage failed.");
    pdal_stage_reset(m_rustStage);
    return true;
}

} // namespace pdal
