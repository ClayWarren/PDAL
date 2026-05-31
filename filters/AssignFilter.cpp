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

    pdal_point_layout_t* rustLayout = rust_view_converter::toRustLayout(layout);
    for (const std::string& value : m_args->m_valueStrings)
    {
        std::string target = rust_view_converter::takeString(
            pdal_assign_statement_target_dim(value.c_str()));
        if (target.empty())
        {
            pdal_point_layout_destroy(rustLayout);
            rust_view_converter::throwLastError(
                "Invalid assign value expression.");
        }

        if (layout->findDim(target) == Dimension::Id::Unknown)
        {
            layout->registerOrAssignDim(target, Dimension::Type::Double);
            pdal_point_layout_destroy(rustLayout);
            rustLayout = rust_view_converter::toRustLayout(layout);
        }

        if (!pdal_stage_validate_assign_statement_with_layout(value.c_str(),
                                                              rustLayout))
        {
            pdal_point_layout_destroy(rustLayout);
            rust_view_converter::throwLastError(
                "Invalid assign value expression.");
        }
    }
    pdal_point_layout_destroy(rustLayout);
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

    if (!m_args->m_valueStrings.empty())
    {
        pdal_point_view_t* rustPoint =
            rust_view_converter::toRustPoint(point, point.layout());
        std::vector<const char*> valuePtrs;
        valuePtrs.reserve(m_args->m_valueStrings.size());
        for (const std::string& value : m_args->m_valueStrings)
            valuePtrs.push_back(value.c_str());
        if (!pdal_point_view_apply_assign_statements(
                rustPoint, valuePtrs.data(), valuePtrs.size(), nullptr, 0))
        {
            pdal_point_view_destroy(rustPoint);
            rust_view_converter::throwLastError(
                "Rust assign value expression failed.");
        }
        rust_view_converter::fromRustPoint(rustPoint, 0, point);
        pdal_point_view_destroy(rustPoint);
    }

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

    if (!m_args->m_valueStrings.empty())
    {
        pdal_point_view_t* rustView = rust_view_converter::toRust(view);
        std::vector<const char*> valuePtrs;
        valuePtrs.reserve(m_args->m_valueStrings.size());
        for (const std::string& value : m_args->m_valueStrings)
            valuePtrs.push_back(value.c_str());
        if (!pdal_point_view_apply_assign_statements(
                rustView, valuePtrs.data(), valuePtrs.size(), nullptr, 0))
        {
            pdal_point_view_destroy(rustView);
            rust_view_converter::throwLastError(
                "Rust assign value expression failed.");
        }
        rust_view_converter::fromRust(rustView, view);
        pdal_point_view_destroy(rustView);
    }
}

} // namespace pdal
