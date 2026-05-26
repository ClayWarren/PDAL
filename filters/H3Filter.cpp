/******************************************************************************
 * Copyright (c) 2024, Howard Butler (info@hobu.co)
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

#include "H3Filter.hpp"

#include <pdal/util/ProgramArgs.hpp>
#include <pdal/util/Utils.hpp>

#include <pdal/private/SrsTransform.hpp>
#include <pdal_capi.h>

#include <cctype>
#include <limits>
#include <map>
#include <string>
#include <vector>

#include <h3api.h>

#include <pdal/private/RustViewConverter.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{"filters.h3",
                                     "Compute H3 indexes for points",
                                     "https://pdal.org/stages/filters.h3.html"};

CREATE_STATIC_STAGE(H3Filter, s_info)

std::string H3Filter::getName() const
{
    return s_info.name;
}

struct H3Filter::Args
{
    int m_resolution;
};

H3Filter::H3Filter() : m_args(new Args), m_rustStage(nullptr) {}

H3Filter::~H3Filter()
{
    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
}

void H3Filter::addArgs(ProgramArgs& args)
{
    // Set resolution and such
    args.add("resolution", "H3 resolution parameter", m_args->m_resolution)
        .setPositional();
}

void H3Filter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Dimension::Id::H3);
    m_layout = layout;
}

void H3Filter::prepared(PointTableRef table)
{
    (void)table;
    pdal_stage_t* stage = pdal_stage_create_h3(m_args->m_resolution);
    if (!stage)
        rust_view_converter::throwLastError("Rust C ABI call failed.");
    pdal_stage_destroy(stage);
}

bool H3Filter::processOne(PointRef& point)
{
    if (!m_rustStage)
    {
        m_rustStage = pdal_stage_create_h3(m_args->m_resolution);
        if (!m_rustStage)
            rust_view_converter::throwLastError(
                "Unable to create Rust H3 stage.");
    }

    pdal_point_view_t* rustView =
        rust_view_converter::toRustPoint(point, m_layout);
    pdal_stage_process_one_at(m_rustStage, rustView, 0);
    rust_view_converter::fromRustPoint(rustView, 0, point);
    pdal_point_view_destroy(rustView);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError("Rust H3 stage failed.");
    pdal_stage_reset(m_rustStage);
    return true;
}

void H3Filter::spatialReferenceChanged(const SpatialReference& srs)
{
    createTransform(srs);
}

void H3Filter::createTransform(const SpatialReference& srsSRS)
{
    if (srsSRS.empty())
        throwError("source data has no spatial reference");

    m_transform.reset(new SrsTransform(srsSRS, "EPSG:4326"));
}

void H3Filter::filter(PointView& view)
{
    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
    m_rustStage = pdal_stage_create_h3(m_args->m_resolution);
    if (!m_rustStage)
        rust_view_converter::throwLastError("Unable to create Rust H3 stage.");
    rust_view_converter::runInPlace(m_rustStage, view);
}

} // namespace pdal
