/******************************************************************************
 * Copyright (c) 2020, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "LloydKMeansFilter.hpp"

#include "private/RustViewConverter.hpp"

#include <pdal_capi.h>

namespace pdal
{
using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.lloydkmeans",
    "Extract and label clusters using K-means (Lloyd's algorithm).",
    "https://pdal.org/stages/filters.lloydkmeans.html"};

CREATE_STATIC_STAGE(LloydKMeansFilter, s_info)

LloydKMeansFilter::LloydKMeansFilter() {}

std::string LloydKMeansFilter::getName() const
{
    return s_info.name;
}

void LloydKMeansFilter::addArgs(ProgramArgs& args)
{
    args.add("k", "Number of clusters to segment", m_k,
             static_cast<uint16_t>(10));
    args.add("dimensions", "Dimensions to cluster", m_dimStringList,
             {"X", "Y", "Z"});
    args.add("maxiters", "Maximum number of iterations", m_maxiters,
             static_cast<uint16_t>(10));
}

void LloydKMeansFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Id::ClusterID);
}

void LloydKMeansFilter::prepared(PointTableRef table)
{
    const PointLayoutPtr layout(table.layout());

    if (m_dimStringList.size())
    {
        for (std::string& s : m_dimStringList)
        {
            Dimension::Id id = layout->findDim(s);
            if (id == Dimension::Id::Unknown)
                throwError("Invalid dimension '" + s +
                           "' specified for "
                           "'dimensions' option.");
            m_dimIdList.push_back(id);
        }
    }
}

void LloydKMeansFilter::filter(PointView& view)
{
    std::vector<std::string> dims;
    std::vector<const char*> dimNames;
    for (auto dim : m_dimIdList)
    {
        dims.push_back(view.layout()->dimName(dim));
        dimNames.push_back(dims.back().c_str());
    }

    pdal_stage_t* stage = pdal_stage_create_lloydkmeans(
        m_k, m_maxiters, dimNames.data(), dimNames.size());
    if (!stage)
        throwError("Failed to create Rust Lloyd K-means stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
