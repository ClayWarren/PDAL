/******************************************************************************
 * Copyright (c) 2021, Preston J. Hartzell (preston.hartzell@gmail.com)
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

#include "GpsTimeConvert.hpp"

#include <pdal_capi.h>

#include <pdal/private/RustViewConverter.hpp>

namespace pdal
{

static PluginInfo const s_info{
    "filters.gpstimeconvert",
    "Convert between GPS Time, GPS Standard Time, and GPS Week Seconds",
    "https://pdal.org/stages/filters.gpstimeconvert.html"};

CREATE_STATIC_STAGE(GpsTimeConvert, s_info)

std::string GpsTimeConvert::getName() const
{
    return s_info.name;
}

GpsTimeConvert::~GpsTimeConvert()
{
    pdal_stage_destroy(m_rustStage);
}

void GpsTimeConvert::addArgs(ProgramArgs& args)
{
    args.add("conversion", "conversion (deprecated)", m_conversion);
    args.add("in_time", "input time type", m_inTime).setPositional();
    args.add("out_time", "output time type", m_outTime).setPositional();
    args.add("start_date", "GMT start date of data in 'YYYY-MM-DD' format",
             m_strDate, "");
    args.add(
        "wrap",
        "reset output week seconds to zero on Sundays, day second at midnight",
        m_wrap, false);
    args.add(
        "wrapped",
        "input weeks seconds reset to zero on Sundays, day second at midnight",
        m_wrapped, false);
    args.add("wrapped_tolerance", "tolerance when unwrapping",
             m_wrappedTolerance, 1.0);
}

void GpsTimeConvert::initialize()
{
    pdal_options_t* ops = pdal_options_create();
    pdal_options_add_str(ops, "conversion", m_conversion.c_str());
    pdal_options_add_str(ops, "in_time", m_inTime.c_str());
    pdal_options_add_str(ops, "out_time", m_outTime.c_str());
    pdal_options_add_str(ops, "start_date", m_strDate.c_str());
    pdal_options_add_str(ops, "wrap", m_wrap ? "true" : "false");
    pdal_options_add_str(ops, "wrapped", m_wrapped ? "true" : "false");
    pdal_options_add_f64(ops, "wrapped_tolerance", m_wrappedTolerance);

    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
    m_rustStage = pdal_stage_create_gpstimeconvert(ops);
    pdal_options_destroy(ops);

    if (!m_rustStage)
        rust_view_converter::throwLastError("Rust C ABI call failed.");
}

void GpsTimeConvert::prepared(PointTableRef table)
{
    m_layout = table.layout();
}

void GpsTimeConvert::ready(PointTableRef)
{
    if (m_rustStage)
        pdal_stage_reset(m_rustStage);
}

bool GpsTimeConvert::processOne(PointRef& point)
{
    pdal_point_view_t* rustPoint =
        rust_view_converter::toRustPoint(point, m_layout);
    bool keep = pdal_stage_process_one_at(m_rustStage, rustPoint, 0);
    if (keep)
        rust_view_converter::fromRustPoint(rustPoint, 0, point);
    pdal_point_view_destroy(rustPoint);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError("Rust GPS time conversion failed.");
    return keep;
}

PointViewSet GpsTimeConvert::run(PointViewPtr view)
{
    PointViewSet viewSet;
    viewSet.insert(rust_view_converter::runSingle(m_rustStage, view));
    return viewSet;
}

void GpsTimeConvert::filter(PointView& view)
{
    rust_view_converter::runInPlace(m_rustStage, view);
}

} // namespace pdal
