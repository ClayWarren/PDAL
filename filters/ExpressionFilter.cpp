/******************************************************************************
 * Copyright (c) 2023, Howard Butler (info@hobu.co)
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

#include "ExpressionFilter.hpp"

#include "./private/expr/ConditionalExpression.hpp"
#include <pdal/util/ProgramArgs.hpp>
#include <pdal/util/Utils.hpp>
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

#include <cctype>
#include <limits>
#include <map>
#include <string>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.expression", "Filter points satisfy a conditional expression.",
    "https://pdal.org/stages/filters.expression.html"};

CREATE_STATIC_STAGE(ExpressionFilter, s_info)

std::string ExpressionFilter::getName() const
{
    return s_info.name;
}

struct ExpressionFilter::Args
{
    std::vector<expr::ConditionalExpression> m_expressions;
};

ExpressionFilter::ExpressionFilter() : m_args(new Args) {}

ExpressionFilter::~ExpressionFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

void ExpressionFilter::addArgs(ProgramArgs& args)
{
    args.add("expression", "Conditional expression describing points to keep",
             m_args->m_expressions)
        .setPositional();
    args.addSynonym("expression", "limits");
}

void ExpressionFilter::initialize()
{
    std::vector<std::string> exprs;
    for (auto const& e : m_args->m_expressions)
        exprs.push_back(e.print());

    std::vector<const char*> sources;
    for (auto const& s : exprs)
        sources.push_back(s.c_str());

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    m_rust_stage = pdal_stage_create_expression(sources.data(), sources.size());
    if (!m_rust_stage)
    {
        std::string err = pdal_last_error();
        if (!err.empty())
            throwError(err);
    }
}

void ExpressionFilter::ready(PointTableRef table)
{
    if (m_rust_stage)
        pdal_stage_reset(m_rust_stage);
}

void ExpressionFilter::prepared(PointTableRef table)
{
    for (auto& expression : m_args->m_expressions)
    {
        if (!expression.valid())
        {
            Utils::StatusWithReason status = expression.prepare(table.layout());
            if (!status)
                throwError(status.what());
        }
    }
}

bool ExpressionFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return false;
}

PointViewSet ExpressionFilter::run(PointViewPtr inView)
{
    PointViewSet viewSet;
    if (m_rust_stage)
    {
        pdal_point_view_t* outputs[100];
        uint64_t num_outputs = pdal_stage_run_multi(m_rust_stage, (pdal_point_view_t*)inView.get(), outputs, 100);
        for (uint64_t i = 0; i < num_outputs; ++i)
        {
            viewSet.insert(PointViewPtr((PointView*)outputs[i]));
        }
    }
    return viewSet;
}

} // namespace pdal
