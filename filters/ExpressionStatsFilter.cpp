/******************************************************************************
 * Copyright (c) 2024, Howard Butler (info@hobu.co)
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

#include "ExpressionStatsFilter.hpp"

#include "private/RustMetadata.hpp"
#include "private/RustViewConverter.hpp"
#include "./private/expr/ConditionalExpression.hpp"
#include <pdal/util/ProgramArgs.hpp>
#include <pdal/util/Utils.hpp>
#include <pdal_capi.h>

#include <cctype>
#include <limits>
#include <map>
#include <string>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.expressionstats",
    "Accumulate count statistics for a given dimension for an array of "
    "expressions",
    "https://pdal.org/stages/filters.expressionstats.html"};

CREATE_STATIC_STAGE(ExpressionStatsFilter, s_info)

std::string ExpressionStatsFilter::getName() const
{
    return s_info.name;
}

struct ExpressionStatsFilter::Args
{
    std::vector<expr::ConditionalExpression> m_expressions;
    std::string m_dimName;
    Arg* m_whereArg;
};

ExpressionStatsFilter::ExpressionStatsFilter() : m_args(new Args) {}

ExpressionStatsFilter::~ExpressionStatsFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

void ExpressionStatsFilter::addArgs(ProgramArgs& args)
{
    m_args->m_whereArg = &args.add("expressions",
                                   "Conditional expressions describing points "
                                   "to be passed to this filter",
                                   m_args->m_expressions)
                              .setPositional();
    args.add("dimension",
             "Dimension on which apply expression to calculate statistics",
             m_args->m_dimName)
        .setPositional();
}

void ExpressionStatsFilter::initialize()
{
    std::vector<std::string> expressionStrings;
    expressionStrings.reserve(m_args->m_expressions.size());
    for (const auto& expression : m_args->m_expressions)
        expressionStrings.push_back(expression.print());

    std::vector<const char*> exprs;
    exprs.reserve(expressionStrings.size());
    for (const auto& s : expressionStrings)
        exprs.push_back(s.c_str());

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
    m_rust_stage = pdal_stage_create_expressionstats(m_args->m_dimName.c_str(),
                                                     exprs.data(), exprs.size());
    if (!m_rust_stage)
        throwError(pdal_last_error());
}

void ExpressionStatsFilter::prepared(PointTableRef table)
{
    m_layout = table.layout();
}

bool ExpressionStatsFilter::processOne(PointRef& point)
{
    pdal_point_view_t* rustPoint =
        rust_view_converter::toRustPoint(point, m_layout);
    bool keep = pdal_stage_process_one_at(m_rust_stage, rustPoint, 0);
    pdal_point_view_destroy(rustPoint);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError(
            "Rust expressionstats streaming failed.");
    return keep;
}

void ExpressionStatsFilter::filter(PointView& view)
{
    rust_view_converter::runInPlace(m_rust_stage, view);
}

void ExpressionStatsFilter::done(PointTableRef table)
{
    pdal_metadata_node_t* rustMetadata = pdal_stage_metadata(m_rust_stage);
    if (!rustMetadata)
        rust_view_converter::throwLastError("Rust expressionstats metadata failed.");

    rust_metadata::addChildrenTo(m_metadata, rustMetadata);
    pdal_metadata_node_destroy(rustMetadata);
}

} // namespace pdal
