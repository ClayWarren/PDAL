/******************************************************************************
 * Copyright (c) 2016-2017, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "OutlierFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>

#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include <string>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.outlier", "Outlier removal",
    "https://pdal.org/stages/filters.outlier.html"};

CREATE_STATIC_STAGE(OutlierFilter, s_info)

std::string OutlierFilter::getName() const
{
    return s_info.name;
}

void OutlierFilter::addArgs(ProgramArgs& args)
{
    args.add("method", "Method [default: statistical]", m_method,
             "statistical");
    args.add("min_k", "Minimum number of neighbors in radius", m_minK, 2);
    args.add("radius", "Radius", m_radius, 1.0);
    args.add("mean_k", "Mean number of neighbors", m_meanK, 8);
    args.add("multiplier", "Standard deviation threshold", m_multiplier, 2.0);
    args.add("class", "Class to use for noise points", m_class,
             ClassLabel::LowPoint);
}

void OutlierFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Dimension::Id::Classification);
}

PointViewSet OutlierFilter::run(PointViewPtr inView)
{
    PointViewSet viewSet;
    if (!inView->size())
        return viewSet;

    pdal_stage_t* stage = pdal_stage_create_outlier(
        m_method.c_str(), m_minK, m_radius, m_meanK, m_multiplier, m_class);
    if (!stage)
        throwError("Failed to create Rust outlier stage.");

    rust_view_converter::runInPlace(stage, *inView);
    pdal_stage_destroy(stage);
    viewSet.insert(inView);
    return viewSet;
}

} // namespace pdal
