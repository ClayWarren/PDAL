/******************************************************************************
 * Copyright (c) 2020, University Nevada, Reno
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

#include "ZsmoothFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>

#include <pdal_capi.h>

namespace pdal
{

static PluginInfo const ptstatInfo{
    "filters.zsmooth", "Zsmooth Filter",
    "https://pdal.org/stages/filters.zsmooth.html"};

struct ZsmoothFilter::Private
{
    double radius;
    double pos;
    std::string dimName;
    Dimension::Id statDim;
};

CREATE_STATIC_STAGE(ZsmoothFilter, ptstatInfo)

std::string ZsmoothFilter::getName() const
{
    return ptstatInfo.name;
}

ZsmoothFilter::ZsmoothFilter() : m_p(new Private) {}

ZsmoothFilter::~ZsmoothFilter() {}

void ZsmoothFilter::addArgs(ProgramArgs& args)
{
    args.add("radius",
             "Radius in X/Y plane in which to find neighboring points",
             m_p->radius, 1.0);
    args.add("medianpercent",
             "Location (percent) in neighbor list at which to find "
             "neighbor Z value (min == 0, max == 100, median == 50, etc.)",
             m_p->pos, 50.0);
    args.add("dim", "Name of dimension in which to store statistic",
             m_p->dimName)
        .setPositional();
}

void ZsmoothFilter::addDimensions(PointLayoutPtr layout)
{
    m_p->statDim =
        layout->registerOrAssignDim(m_p->dimName, Dimension::Type::Double);
    if (m_p->statDim == Dimension::Id::Z)
        throwError("Can't use 'Z' as output dimension.");
}

void ZsmoothFilter::prepared(PointTableRef)
{
    if (m_p->pos < 0.0 || m_p->pos > 100.0)
        throwError("'medicanpercent' value must be in the range [0, 100]");
    m_p->pos /= 100.0;
}

void ZsmoothFilter::filter(PointView& view)
{
    pdal_stage_t* stage =
        pdal_stage_create_zsmooth(m_p->radius, m_p->pos, m_p->dimName.c_str());
    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
