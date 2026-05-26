/******************************************************************************
 * Copyright (c) 2017, Hobu Inc., info@hobu.co
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
 *
 ****************************************************************************/

#include "AssignFilter.hpp"
#include "private/DimRange.hpp"
#include "private/expr/AssignStatement.hpp"
#include <pdal/StageFactory.hpp>
#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.assign",
    "Assign values for a dimension range to a specified value.",
    "https://pdal.org/stages/filters.assign.html"};

CREATE_STATIC_STAGE(AssignFilter, s_info)

struct AssignRange : public DimRange
{
    void parse(const std::string& r);
    double m_value;
};

struct AssignArgs
{
    std::vector<AssignRange> m_assignments;
    DimRange m_condition;
    std::vector<std::string> m_valueStrings;
    std::vector<expr::AssignStatement> m_statements;
};

void AssignRange::parse(const std::string& r)
{
    std::string::size_type pos, count;

    pos = subParse(r);
    count = Utils::extractSpaces(r, pos);
    pos += count;

    if (r[pos] != '=')
        throw error("Missing '=' assignment separator.");
    pos++;

    count = Utils::extractSpaces(r, pos);
    pos += count;

    // Extract value.
    Utils::StringStreamClassicLocale ss(r.data() + pos);
    auto start = ss.tellg();
    ss >> m_value;
    if (ss.fail())
        throw error("Missing value to assign following '='.");
    else if (ss.eof())
        pos = r.size();
    else
    {
        pos += (ss.tellg() - start);
        count = Utils::extractSpaces(r, pos);
        pos += count;
    }

    if (pos != r.size())
        throw error("Invalid characters following valid range.");
}

std::istream& operator>>(std::istream& in, AssignRange& r)
{
    std::string s;

    std::getline(in, s);
    try
    {
        r.parse(s);
    }
    catch (DimRange::error&)
    {
        in.setstate(std::ios_base::failbit);
    }
    return in;
}

std::ostream& operator<<(std::ostream& out, const AssignRange& r)
{
    out << (const DimRange&)r;
    out << "=" << r.m_name;
    return out;
}

AssignFilter::AssignFilter() : m_args(new AssignArgs) {}

AssignFilter::~AssignFilter() {}

void AssignFilter::addArgs(ProgramArgs& args)
{
    args.add("assignment", "Values to assign to dimensions based on range.",
             m_args->m_assignments);
    args.add("condition", "Condition for assignment based on range.",
             m_args->m_condition);
    args.add("value", "Value to assign to dimension based on expression.",
             m_args->m_valueStrings);
}

void AssignFilter::prepared(PointTableRef table)
{
    PointLayoutPtr layout(table.layout());

    m_args->m_condition.m_id = layout->findDim(m_args->m_condition.m_name);
    for (auto& r : m_args->m_assignments)
    {
        r.m_id = layout->findDim(r.m_name);
        if (r.m_id == Dimension::Id::Unknown)
            throwError("Invalid dimension name in 'assignment' option: '" +
                       r.m_name + "'.");
    }

    m_args->m_statements.clear();
    for (const std::string& value : m_args->m_valueStrings)
    {
        if (!pdal_stage_validate_assign_statement(value.c_str()))
            rust_view_converter::throwLastError("Rust C ABI call failed.");

        expr::AssignStatement stmt;
        Utils::StatusWithReason status = Utils::fromString(value, stmt);
        if (!status)
            throwError(status.what());
        m_args->m_statements.push_back(std::move(stmt));
    }

    for (expr::AssignStatement& expr : m_args->m_statements)
    {
        expr::IdentExpression& ident = expr.identExpr();
        if (!expr.prepare(layout) && ident.eval() == Dimension::Id::Unknown)
            layout->registerOrAssignDim(ident.name(), Dimension::Type::Double);

        // Try to prepare again after potentially adding a dimension.
        auto status = expr.prepare(layout);
        if (!status)
            throwError(status.what());
    }
}

bool AssignFilter::processOne(PointRef& point)
{
    if (!m_args->m_condition.m_name.empty() &&
        !m_args->m_condition.valuePasses(
            point.getFieldAs<double>(m_args->m_condition.m_id)))
        return true;

    for (const auto& r : m_args->m_assignments)
        if (r.valuePasses(point.getFieldAs<double>(r.m_id)))
            point.setField(r.m_id, r.m_value);

    for (expr::AssignStatement& expr : m_args->m_statements)
        if (expr.conditionalExpr().eval(point))
            point.setField(expr.identExpr().eval(),
                           expr.valueExpr().eval(point));

    return true;
}

void AssignFilter::filter(PointView& view)
{
    bool has_condition = !m_args->m_condition.m_name.empty();
    const char* cond_dim =
        has_condition ? m_args->m_condition.m_name.c_str() : nullptr;

    std::vector<pdal_assign_range_t> assignments;
    for (const auto& r : m_args->m_assignments)
    {
        pdal_assign_range_t range;
        range.dim_name = r.m_name.c_str();
        range.value = r.m_value;
        range.lower_bound = r.m_lower_bound;
        range.upper_bound = r.m_upper_bound;
        range.inclusive_lower = r.m_inclusive_lower_bound;
        range.inclusive_upper = r.m_inclusive_upper_bound;
        range.negate = r.m_negate;
        assignments.push_back(range);
    }

    if (has_condition || !assignments.empty())
    {
        pdal_stage_t* stage = pdal_stage_create_assign(
            has_condition, cond_dim, m_args->m_condition.m_lower_bound,
            m_args->m_condition.m_upper_bound,
            m_args->m_condition.m_inclusive_lower_bound,
            m_args->m_condition.m_inclusive_upper_bound,
            m_args->m_condition.m_negate,
            assignments.empty() ? nullptr : assignments.data(),
            assignments.size());
        if (!stage)
            throwError("Failed to create Rust assign stage.");

        rust_view_converter::runInPlace(stage, view);
        pdal_stage_destroy(stage);
    }

    for (PointId id = 0; id < view.size(); ++id)
    {
        PointRef point(view, id);
        for (expr::AssignStatement& expr : m_args->m_statements)
            if (expr.conditionalExpr().eval(point))
                point.setField(expr.identExpr().eval(),
                               expr.valueExpr().eval(point));
    }
}

} // namespace pdal
