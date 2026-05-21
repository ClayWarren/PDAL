/******************************************************************************
 * Copyright (c) 2018, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "ReturnsFilter.hpp"
#include <pdal/private/RustViewConverter.hpp>

#include <pdal/util/ProgramArgs.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.returns", "Split data by return order",
    "https://pdal.org/stages/filters.returns.html"};

CREATE_STATIC_STAGE(ReturnsFilter, s_info)

std::string ReturnsFilter::getName() const
{
    return s_info.name;
}

void ReturnsFilter::addArgs(ProgramArgs& args)
{
    args.add("groups",
             "Comma-separated list of return number groupings ('first', "
             "'last', 'intermediate', or 'only')",
             m_returnsString, {"last"});
}

void ReturnsFilter::prepared(PointTableRef table)
{
    const PointLayoutPtr layout(table.layout());
    if (!layout->hasDim(Dimension::Id::ReturnNumber) ||
        !layout->hasDim(Dimension::Id::NumberOfReturns))
    {
        log()->get(LogLevel::Warning)
            << "Could not find ReturnNumber or "
               "NumberOfReturns. Proceeding with all returns.\n";
    }
}

PointViewSet ReturnsFilter::run(PointViewPtr inView)
{
    std::vector<const char*> groups;
    for (const auto& r : m_returnsString)
        groups.push_back(r.c_str());

    pdal_stage_t* stage =
        pdal_stage_create_returns(groups.data(), groups.size());
    if (!stage)
        throwError("Failed to create Rust returns stage.");

    PointViewSet viewSet = rust_view_converter::runMulti(stage, inView, 4);
    pdal_stage_destroy(stage);

    return viewSet;
}

} // namespace pdal
