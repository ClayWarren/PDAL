/******************************************************************************
 * Copyright (c) 2016, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "LOFFilter.hpp"

#include "private/RustViewConverter.hpp"

#include <pdal_capi.h>

#include <string>

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.lof", "LOF Filter", "https://pdal.org/stages/filters.lof.html"};

CREATE_STATIC_STAGE(LOFFilter, s_info)

std::string LOFFilter::getName() const
{
    return s_info.name;
}

void LOFFilter::addArgs(ProgramArgs& args)
{
    args.add("minpts", "Minimum number of points", m_minpts, (size_t)10);
}

void LOFFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Id::NNDistance);
    layout->registerDim(Id::LocalReachabilityDistance);
    layout->registerDim(Id::LocalOutlierFactor);
}

void LOFFilter::filter(PointView& view)
{
    log()->get(LogLevel::Debug) << "Computing k-distances...\n";
    log()->get(LogLevel::Debug) << "Computing lrd...\n";
    log()->get(LogLevel::Debug) << "Computing LOF...\n";

    pdal_stage_t* stage = pdal_stage_create_lof(m_minpts);
    if (!stage)
        throwError("Failed to create Rust lof stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
