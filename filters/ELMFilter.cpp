/******************************************************************************
 * Copyright (c) 2017, Bradley J Chambers (brad.chambers@gmail.com)
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

// PDAL implementation of the Extended Local Minimum (ELM) method as published
// in Z. Chen, B. Devereux, B. Gao, and G. Amable, “Upward-fusion urban DTM
// generating method using airborne Lidar data,” ISPRS J. Photogramm. Remote
// Sens., vol. 72, pp. 121–130, 2012.

#include "ELMFilter.hpp"

#include "private/RustViewConverter.hpp"

#include <pdal_capi.h>

#include <string>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.elm", "Marks low points as noise.",
    "https://pdal.org/stages/filters.elm.html"};

CREATE_STATIC_STAGE(ELMFilter, s_info)

std::string ELMFilter::getName() const
{
    return s_info.name;
}

void ELMFilter::addArgs(ProgramArgs& args)
{
    args.add("cell", "Cell size", m_cell, 10.0);
    args.add("class", "Class to use for noise points", m_class,
             ClassLabel::LowPoint);
    args.add("threshold", "Threshold value", m_threshold, 1.0);
}

void ELMFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Dimension::Id::Classification);
}

void ELMFilter::filter(PointView& view)
{
    pdal_stage_t* stage = pdal_stage_create_elm(m_cell, m_class, m_threshold);
    if (!stage)
        throwError("Failed to create Rust ELM stage.");
    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
