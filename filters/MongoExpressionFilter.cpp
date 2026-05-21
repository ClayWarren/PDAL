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

#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include <pdal/private/RustViewConverter.hpp>

namespace pdal
{

static const StaticPluginInfo s_info{
    "filters.mongo", "Pass only points that pass a logic filter.",
    "https://pdal.org/stages/filters.mongo.html"};

CREATE_STATIC_STAGE(MongoExpressionFilter, s_info);

struct MongoExpressionFilter::Args
{
    std::string m_json;
};

std::string MongoExpressionFilter::getName() const
{
    return s_info.name;
}

MongoExpressionFilter::MongoExpressionFilter()
    : m_args(new Args())
{
}

MongoExpressionFilter::~MongoExpressionFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

void MongoExpressionFilter::addArgs(ProgramArgs& args)
{
    args.add("expression", "Logical query expression", m_args->m_json)
        .setPositional();
}

void MongoExpressionFilter::initialize()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
    m_rust_stage = pdal_stage_create_mongoexpression(m_args->m_json.c_str());
    if (!m_rust_stage)
        throwError(pdal_last_error());
}

void MongoExpressionFilter::prepared(PointTableRef table)
{
    // Captured so streaming processOne can convert a single point.
    m_layout = table.layout();
}

PointViewSet MongoExpressionFilter::run(PointViewPtr inView)
{
    return rust_view_converter::runMulti(m_rust_stage, inView, 1);
}

bool MongoExpressionFilter::processOne(PointRef& pr)
{
    pdal_point_view_t* rustPoint =
        rust_view_converter::toRustPoint(pr, m_layout);
    bool keep = pdal_stage_process_one_at(m_rust_stage, rustPoint, 0);
    pdal_point_view_destroy(rustPoint);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError(
            "Rust mongo expression streaming failed.");
    return keep;
}

} // namespace pdal
