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

#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include <pdal/private/RustViewConverter.hpp>

#include <string>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.expression", "Pass only points given an expression",
    "https://pdal.org/stages/filters.expression.html"};

CREATE_STATIC_STAGE(ExpressionFilter, s_info)

std::string ExpressionFilter::getName() const
{
    return s_info.name;
}

struct ExpressionFilter::Args
{
    std::vector<std::string> m_expressions;
    Arg* m_whereArg;
};

ExpressionFilter::ExpressionFilter() : m_args(new Args) {}

ExpressionFilter::~ExpressionFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

void ExpressionFilter::addArgs(ProgramArgs& args)
{
    m_args->m_whereArg = &args.add("expression",
                                   "Conditional expression describing points "
                                   "to be passed to this filter",
                                   m_args->m_expressions)
                              .setPositional();

    args.addSynonym("expression", "limits");
}

void ExpressionFilter::initialize()
{
    if (m_args->m_expressions.empty())
        throwError("No expressions provided.");

    std::vector<std::string> expressionStrings;
    expressionStrings.reserve(m_args->m_expressions.size());
    for (const auto& expression : m_args->m_expressions)
        expressionStrings.push_back(expression);

    std::vector<const char*> exprs;
    exprs.reserve(expressionStrings.size());
    for (const auto& s : expressionStrings)
        exprs.push_back(s.c_str());

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
    m_rust_stage = pdal_stage_create_expression(exprs.data(), exprs.size());
    if (!m_rust_stage)
        throwError(pdal_last_error());
}

void ExpressionFilter::prepared(PointTableRef table)
{
    // Captured so streaming processOne can convert a single point.
    m_layout = table.layout();
}

bool ExpressionFilter::processOne(PointRef& point)
{
    if (m_args->m_expressions.size() != 1)
        throwError(
            "Streaming of expressions only works with a single expression");

    pdal_point_view_t* rustPoint =
        rust_view_converter::toRustPoint(point, m_layout);
    bool keep = pdal_stage_process_one_at(m_rust_stage, rustPoint, 0);
    pdal_point_view_destroy(rustPoint);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError(
            "Rust expression streaming failed.");
    return keep;
}

PointViewSet ExpressionFilter::run(PointViewPtr inView)
{
    // One output view per expression.
    return rust_view_converter::runMulti(m_rust_stage, inView,
                                         m_args->m_expressions.size());
}

} // namespace pdal
