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
 *     * Neither the name of the Andrew Bell or libLAS nor the names of
 *       its contributors may be used to endorse or promote products derived
 *       from this software without specific prior written permission.
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

#include "TailFilter.hpp"

#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.tail", "Return N points from end of the point cloud.",
    "https://pdal.org/stages/filters.tail.html"};

CREATE_STATIC_STAGE(TailFilter, s_info)

TailFilter::~TailFilter()
{
    pdal_stage_destroy(m_rust_stage);
}

std::string TailFilter::getName() const
{
    return s_info.name;
}

void TailFilter::addArgs(ProgramArgs& args)
{
    args.add("count",
             "Number of points to return from end. "
             "If 'invert' is true, number of points to drop from the end.",
             m_count, point_count_t(10));
    args.add("invert",
             "If true, 'count' specifies the number of points "
             "at the end to drop.",
             m_invert, false);
}

void TailFilter::initialize()
{
    pdal_options_t* ops = pdal_options_create();
    pdal_options_add_u64(ops, "count", m_count);
    pdal_options_add_str(ops, "invert", m_invert ? "true" : "false");

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    m_rust_stage = pdal_stage_create_tail(ops);
    pdal_options_destroy(ops);
}

PointViewSet TailFilter::run(PointViewPtr inView)
{
    if (m_count > inView->size())
        log()->get(LogLevel::Warning)
            << "Requested number of points (count=" << m_count
            << ") exceeds number of available points.\n";

    PointViewSet viewSet;
    viewSet.insert(rust_view_converter::runSingle(m_rust_stage, inView));
    return viewSet;
}

} // namespace pdal
