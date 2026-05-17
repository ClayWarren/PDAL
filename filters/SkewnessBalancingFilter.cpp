/******************************************************************************
 * Copyright (c) 2019, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "SkewnessBalancingFilter.hpp"

#include "private/RustViewConverter.hpp"

#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

namespace pdal
{

static PluginInfo const s_info{
    "filters.skewnessbalancing", "Bartels & Wei Skewness Balancing",
    "https://pdal.org/stages/filters.skewnessbalancing.html"};

CREATE_STATIC_STAGE(SkewnessBalancingFilter, s_info)

std::string SkewnessBalancingFilter::getName() const
{
    return s_info.name;
}

void SkewnessBalancingFilter::addArgs(ProgramArgs& args)
{
    args.add("ground_class",
             "Classification value of ground points."
             " [Default: 2]",
             m_groundClass, ClassLabel::Ground);
    args.add("other_class",
             "Classification value of non-ground points."
             " [Default: 1]",
             m_otherClass, ClassLabel::Unclassified);
    args.add("only_ground",
             "Set to true to only modify the CLassification"
             " value of detected ground points. [Default: false]",
             m_onlyGround, false);
}

void SkewnessBalancingFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Dimension::Id::Classification);
}

void SkewnessBalancingFilter::prepared(PointTableRef table)
{
    if ((m_groundClass == m_otherClass) && !m_onlyGround)
    {
        throwError("Ground and non-ground class cannot be"
                   "equal when only_ground is false.");
    }
}

PointViewSet SkewnessBalancingFilter::run(PointViewPtr input)
{
    pdal_stage_t* stage = pdal_stage_create_skewnessbalancing(
        m_groundClass, m_otherClass, m_onlyGround);
    if (!stage)
        throwError("Failed to create Rust skewness balancing stage.");

    PointViewSet viewSet;
    viewSet.insert(rust_view_converter::runSingle(stage, input));
    pdal_stage_destroy(stage);
    return viewSet;
}

} // namespace pdal
