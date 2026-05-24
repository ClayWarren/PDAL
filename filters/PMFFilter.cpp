/******************************************************************************
 * Copyright (c) 2015-2017, 2020 Bradley J Chambers (brad.chambers@gmail.com)
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

// PDAL implementation of K. Zhang, S.-C. Chen, D. Whitman, M.-L. Shyu, J. Yan,
// and C. Zhang, “A progressive morphological filter for removing nonground
// measurements from airborne LIDAR data,” Geosci. Remote Sensing, IEEE Trans.,
// vol. 41, no. 4, pp. 872–882, 2003.

#include "PMFFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include "private/DimRange.hpp"

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.pmf", "Progressive morphological filter",
    "https://pdal.org/stages/filters.pmf.html"};

struct PMFArgs
{
    double m_cellSize;
    bool m_exponential;
    std::vector<DimRange> m_ignored;
    double m_initialDistance;
    StringList m_returns;
    double m_maxDistance;
    double m_maxWindowSize;
    double m_slope;
};

CREATE_STATIC_STAGE(PMFFilter, s_info)

PMFFilter::PMFFilter() : m_args(new PMFArgs) {}

PMFFilter::~PMFFilter() {}

std::string PMFFilter::getName() const
{
    return s_info.name;
}

void PMFFilter::addArgs(ProgramArgs& args)
{
    args.add("cell_size", "Cell size", m_args->m_cellSize, 1.0);
    args.add("exponential", "Exponential growth of window size?",
             m_args->m_exponential, true);
    args.add("ignore", "Ignore values", m_args->m_ignored);
    args.add("initial_distance", "Initial distance", m_args->m_initialDistance,
             0.15);
    args.add("returns", "Include only returns?", m_args->m_returns,
             {"last", "only"});
    args.add("max_distance", "Maximum distance", m_args->m_maxDistance, 2.5);
    args.add("max_window_size", "Maximum window size", m_args->m_maxWindowSize,
             33.0);
    args.add("slope", "Slope", m_args->m_slope, 1.0);
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

void PMFFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Id::Classification);
}

void PMFFilter::prepared(PointTableRef table)
{
    const PointLayoutPtr layout(table.layout());

    if ((m_groundClass == m_otherClass) && !m_onlyGround)
    {
        throwError("Ground and non-ground class cannot be"
                   "equal when only_ground is false.");
    }

    for (auto& r : m_args->m_ignored)
    {
        r.m_id = layout->findDim(r.m_name);
        if (r.m_id == Id::Unknown)
            throwError("Invalid dimension name in 'ignored' option: '" +
                       r.m_name + "'.");
    }

    std::vector<const char*> returns;
    returns.reserve(m_args->m_returns.size());
    for (const std::string& r : m_args->m_returns)
        returns.push_back(r.c_str());

    pdal_stage_t* stage = pdal_stage_create_pmf(
        m_args->m_cellSize, m_args->m_exponential, m_args->m_initialDistance,
        m_args->m_maxDistance, m_args->m_maxWindowSize, m_args->m_slope,
        m_groundClass, m_otherClass, m_onlyGround, returns.data(),
        returns.size());
    if (!stage)
        rust_view_converter::throwLastError("Failed to create Rust PMF stage.");
    pdal_stage_destroy(stage);

    if (m_args->m_returns.size())
    {
        for (auto& r : m_args->m_returns)
            Utils::trim(r);

        if (!layout->hasDim(Id::ReturnNumber) ||
            !layout->hasDim(Id::NumberOfReturns))
        {
            log()->get(LogLevel::Warning) << "Could not find ReturnNumber and "
                                             "NumberOfReturns. Skipping "
                                             "segmentation of last returns and "
                                             "proceeding with all returns.\n";
            m_args->m_returns.clear();
        }
    }
}

PointViewSet PMFFilter::run(PointViewPtr input)
{
    PointViewSet viewSet{input};

    if (!m_args->m_ignored.empty())
        throwError("Rust PMF path does not yet support the 'ignore' option.");

    std::vector<const char*> returns;
    returns.reserve(m_args->m_returns.size());
    for (const std::string& r : m_args->m_returns)
        returns.push_back(r.c_str());

    pdal_stage_t* stage = pdal_stage_create_pmf(
        m_args->m_cellSize, m_args->m_exponential, m_args->m_initialDistance,
        m_args->m_maxDistance, m_args->m_maxWindowSize, m_args->m_slope,
        m_groundClass, m_otherClass, m_onlyGround, returns.data(),
        returns.size());
    if (!stage)
        rust_view_converter::throwLastError("Failed to create Rust PMF stage.");

    rust_view_converter::runInPlace(stage, *input);
    pdal_stage_destroy(stage);
    return viewSet;
}

} // namespace pdal
