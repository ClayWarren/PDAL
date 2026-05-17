/******************************************************************************
 * Copyright (c) 2018, Connor Manning (connor@hobu.co)
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

#include "MongoExpressionFilter.hpp"

#include <nlohmann/json.hpp>

#include <pdal/PointView.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include "private/mongoexpression/Expression.hpp"
#include <pdal_capi.h>

#include "private/RustViewConverter.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.mongoexpression", "Filter points using a MongoDB-style query.",
    "https://pdal.org/stages/filters.mongoexpression.html"};

CREATE_STATIC_STAGE(MongoExpressionFilter, s_info)

struct MongoExpressionFilter::Args
{
    std::string m_expression;
};

struct MongoExpressionFilter::Private
{
    Expression m_expression;
};

MongoExpressionFilter::MongoExpressionFilter()
    : m_args(new Args()), m_p(new Private())
{
}

MongoExpressionFilter::~MongoExpressionFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

std::string MongoExpressionFilter::getName() const
{
    return s_info.name;
}

void MongoExpressionFilter::addArgs(ProgramArgs& args)
{
    args.add("expression", "Expression to evaluate", m_args->m_expression)
        .setPositional();
}

void MongoExpressionFilter::initialize()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
    m_rust_stage = pdal_stage_create_mongoexpression(m_args->m_expression.c_str());
    if (!m_rust_stage)
    {
        std::string err = pdal_last_error();
        if (!err.empty())
            throwError(err);
    }
}

void MongoExpressionFilter::ready(PointTableRef table)
{
    if (m_rust_stage)
        pdal_stage_reset(m_rust_stage);
}

void MongoExpressionFilter::prepared(PointTableRef table)
{
    try
    {
        m_p->m_expression = Expression(table.layout(), NL::json::parse(m_args->m_expression));
    }
    catch (const std::exception& e)
    {
        throwError(std::string("Error parsing expression: ") + e.what());
    }
}

PointViewSet MongoExpressionFilter::run(PointViewPtr inView)
{
    PointViewSet viewSet;
    if (m_rust_stage)
    {
        viewSet.insert(rust_view_converter::runSingle(m_rust_stage, inView));
    }
    return viewSet;
}
bool MongoExpressionFilter::processOne(PointRef& point)
{
    if (m_rust_stage)
    {
        return pdal_stage_process_one(m_rust_stage, (pdal_point_view_t*)point.view(), point.pointId());
    }
    return false;
}
} // namespace pdal
