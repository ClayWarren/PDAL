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

// Adapted from MIT-licensed implemenation provided by
// https://github.com/intel-isl/Open3D/pull/1038.

#include "DBSCANFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>

#include <pdal_capi.h>

#include <string>
#include <vector>

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.dbscan", "DBSCAN Clustering.",
    "https://pdal.org/stages/filters.dbscan.html"};

CREATE_STATIC_STAGE(DBSCANFilter, s_info)

std::string DBSCANFilter::getName() const
{
    return s_info.name;
}

DBSCANFilter::DBSCANFilter() : Filter() {}

void DBSCANFilter::addArgs(ProgramArgs& args)
{
    args.add("min_points", "Min points per cluster", m_minPoints,
             static_cast<uint64_t>(6));
    args.add("eps", "Epsilon", m_eps, 1.0);
    args.add("dimensions", "Dimensions to cluster", m_dimStringList,
             {"X", "Y", "Z"});
}

void DBSCANFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Id::ClusterID);
}

void DBSCANFilter::prepared(PointTableRef table)
{
    const PointLayoutPtr layout(table.layout());

    if (m_dimStringList.size())
    {
        for (std::string& s : m_dimStringList)
        {
            Id id = layout->findDim(s);
            if (id == Id::Unknown)
                throwError("Invalid dimension '" + s +
                           "' specified for "
                           "'dimensions' option.");
            m_dimIdList.push_back(id);
        }
    }

    std::vector<const char*> dimNamePtrs;
    dimNamePtrs.reserve(m_dimStringList.size());
    for (const std::string& s : m_dimStringList)
        dimNamePtrs.push_back(s.c_str());

    pdal_stage_t* stage = pdal_stage_create_dbscan(
        m_minPoints, m_eps, dimNamePtrs.data(), dimNamePtrs.size());
    if (!stage)
        throwError(pdal_last_error());
    pdal_stage_destroy(stage);
}

void DBSCANFilter::filter(PointView& view)
{
    std::vector<std::string> dimNames;
    std::vector<const char*> dimNamePtrs;
    dimNames.reserve(m_dimIdList.size());
    dimNamePtrs.reserve(m_dimIdList.size());
    for (Dimension::Id id : m_dimIdList)
    {
        dimNames.push_back(view.layout()->dimName(id));
        dimNamePtrs.push_back(dimNames.back().c_str());
    }

    pdal_stage_t* stage = pdal_stage_create_dbscan(
        m_minPoints, m_eps, dimNamePtrs.data(), dimNamePtrs.size());
    if (!stage)
        throwError("Failed to create Rust dbscan stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
