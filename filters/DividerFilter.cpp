/******************************************************************************
 * Copyright (c) 2015, Hobu Inc. (info@hobu.co)
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
 *     * Neither the name of Hobu, Inc. nor the names of its contributors
 *       may be used to endorse or promote products derived from this
 *       software without specific prior written permission.
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
 *
 ****************************************************************************/

#include "DividerFilter.hpp"
#include "./private/expr/ConditionalExpression.hpp"
#include "private/RustViewConverter.hpp"
#include <pdal_capi.h>

extern "C" {
    pdal_stage_t* pdal_stage_create_divider(
        int32_t mode,
        int32_t size_mode,
        uint64_t size,
        const uint8_t* evals,
        uint64_t evals_count
    );
}

namespace pdal
{

struct DividerFilter::Args
{
    expr::ConditionalExpression m_splitExpression;
    Mode m_mode = DividerFilter::Mode::Partition;
    SizeMode m_sizeMode = SizeMode::Count;
    point_count_t m_size = 1;

    Arg* m_cntArg = nullptr;
    Arg* m_capArg = nullptr;
    Arg* m_splitExpressionArg = nullptr;
};

static PluginInfo const s_info{
    "filters.divider",
    "Divide points into approximately equal sized groups based on a simple "
    "scheme",
    "https://pdal.org/stages/filters.divider.html"};

CREATE_STATIC_STAGE(DividerFilter, s_info)

DividerFilter::DividerFilter() : m_args(new Args) {}

DividerFilter::~DividerFilter() {}

std::string DividerFilter::getName() const
{
    return s_info.name;
}

std::istream& operator>>(std::istream& in, DividerFilter::Mode& mode)
{
    std::string s;
    in >> s;

    s = Utils::tolower(s);
    if (s == "round_robin")
        mode = DividerFilter::Mode::RoundRobin;
    else if (s == "partition")
        mode = DividerFilter::Mode::Partition;
    else if (s == "expression")
        mode = DividerFilter::Mode::Expression;
    else
        throw pdal_error("filters.divider: Invalid 'mode' option '" + s +
                         "'. "
                         "Valid options are 'partition' and 'round_robin'");
    return in;
}

std::ostream& operator<<(std::ostream& out, const DividerFilter::Mode& mode)
{
    switch (mode)
    {
    case DividerFilter::Mode::RoundRobin:
        out << "round_robin";
        break;
    case DividerFilter::Mode::Partition:
        out << "partition";
        break;
    case DividerFilter::Mode::Expression:
        out << "expression";
        break;
    }
    return out;
}

void DividerFilter::addArgs(ProgramArgs& args)
{
    args.add("mode",
             "A mode of 'partition' will write sequential points "
             "to an output view until the view meets its predetermined size. "
             "'round_robin' mode will iterate through the output views as it "
             "writes sequential points. A mode of 'split' will output new "
             "views every time a 'capacity' number of points with the given "
             "'expression'"
             "are visited",
             m_args->m_mode, DividerFilter::Mode::Partition);
    m_args->m_cntArg =
        &args.add("count", "Number of output views", m_args->m_size);
    m_args->m_capArg = &args.add("capacity",
                                 "Maximum number of points in each "
                                 "output view",
                                 m_args->m_size);
    m_args->m_splitExpressionArg = &args.add(
        "expression", "Expression to cause split", m_args->m_splitExpression);
}

void DividerFilter::prepared(PointTableRef table)
{
    if (m_args->m_mode == Mode::Expression)
    {
        if (!m_args->m_splitExpression.valid())
        {
            std::stringstream oss;
            oss << "The expression '" << m_args->m_splitExpression
                << "' is invalid";
            throwError(oss.str());
        }

        auto status = m_args->m_splitExpression.prepare(table.layout());
        if (!status)
            throwError(status.what());
    }
}

void DividerFilter::initialize()
{
    if (m_args->m_splitExpressionArg->set())
    {
        m_args->m_mode = DividerFilter::Mode::Expression;

        if (!m_args->m_cntArg->set())
            m_args->m_size =
                1; // Default to 1 if the user didn't specify a break count
    }

    else if (m_args->m_mode == Mode::Partition ||
             m_args->m_mode == Mode::RoundRobin)
    {
        if (m_args->m_cntArg->set() && m_args->m_capArg->set())
            throwError(
                "Can't specify both option 'count' and option 'capacity.");
        if (!m_args->m_cntArg->set() && !m_args->m_capArg->set())
            throwError(
                "Must specify either option 'count' or option 'capacity'.");

        if (m_args->m_cntArg->set())
        {
            m_args->m_sizeMode = SizeMode::Count;
            if (m_args->m_size < 2 || m_args->m_size > 1000)
                throwError("Option 'count' must be in the range [2, 1000].");
        }
        if (m_args->m_capArg->set())
        {
            m_args->m_sizeMode = SizeMode::Capacity;
            if (m_args->m_size == 0)
                throwError("Option 'capacity' must be greater than 0.");
        }
    }
}

PointViewSet DividerFilter::run(PointViewPtr inView)
{
    PointViewSet result;
    if (inView->empty())
        return result;

    int32_t mode_val = 0;
    if (m_args->m_mode == Mode::RoundRobin)
        mode_val = 1;
    else if (m_args->m_mode == Mode::Expression)
        mode_val = 2;

    int32_t size_mode_val = 0;
    if (m_args->m_sizeMode == SizeMode::Capacity)
        size_mode_val = 1;

    std::vector<uint8_t> evals;
    if (m_args->m_mode == Mode::Expression)
    {
        evals.reserve(inView->size());
        for (PointRef point : *inView)
        {
            evals.push_back(m_args->m_splitExpression.eval(point) ? 1 : 0);
        }
    }

    pdal_stage_t* stage = pdal_stage_create_divider(
        mode_val,
        size_mode_val,
        m_args->m_size,
        evals.empty() ? nullptr : evals.data(),
        evals.size()
    );

    pdal_point_view_t* rust_in = rust_view_converter::toRust(inView);

    uint64_t max_outputs = inView->size() + 2;
    std::vector<pdal_point_view_t*> rust_outputs(max_outputs, nullptr);

    uint64_t actual_outputs = pdal_stage_run_multi(
        stage,
        rust_in,
        rust_outputs.data(),
        max_outputs
    );

    for (uint64_t i = 0; i < actual_outputs; ++i)
    {
        PointViewPtr outView = rust_view_converter::fromRust(rust_outputs[i], inView);
        result.insert(outView);
        pdal_point_view_destroy(rust_outputs[i]);
    }

    pdal_point_view_destroy(rust_in);
    pdal_stage_destroy(stage);

    return result;
}

} // namespace pdal
