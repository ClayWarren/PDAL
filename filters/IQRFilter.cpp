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

#include "IQRFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>

#include <pdal_capi.h>

#include <string>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.iqr", "Interquartile Range Filter",
    "https://pdal.org/stages/filters.iqr.html"};

CREATE_STATIC_STAGE(IQRFilter, s_info)

std::string IQRFilter::getName() const
{
    return s_info.name;
}

void IQRFilter::addArgs(ProgramArgs& args)
{
    args.add("k", "Number of deviations", m_multiplier, 1.5);
    args.add("dimension", "Dimension on which to calculate statistics",
             m_dimName);
}

void IQRFilter::prepared(PointTableRef table)
{
    PointLayoutPtr layout(table.layout());
    m_dimId = layout->findDim(m_dimName);
    if (m_dimId == Dimension::Id::Unknown)
        throwError("Dimension '" + m_dimName + "' does not exist.");

    pdal_stage_t* stage =
        pdal_stage_create_iqr(m_multiplier, m_dimName.c_str());
    if (!stage)
        throwError(pdal_last_error());
    pdal_stage_destroy(stage);
}

PointViewSet IQRFilter::run(PointViewPtr view)
{
    pdal_stage_t* stage = pdal_stage_create_iqr(
        m_multiplier, view->layout()->dimName(m_dimId).c_str());
    if (!stage)
        throwError("Failed to create Rust IQR stage.");
    PointViewSet viewSet;
    viewSet.insert(rust_view_converter::runSingle(stage, view));
    pdal_stage_destroy(stage);
    return viewSet;
}

} // namespace pdal
