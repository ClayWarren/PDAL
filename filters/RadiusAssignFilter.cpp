#include "RadiusAssignFilter.hpp"

#include <pdal/PipelineManager.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include "private/DimRange.hpp"

#include <string>
#include <vector>

namespace pdal
{

namespace
{

pdal_range_limit_t toRustLimit(const DimRange& r)
{
    pdal_range_limit_t limit;
    limit.dim_name = r.m_name.c_str();
    limit.lower_bound = r.m_lower_bound;
    limit.upper_bound = r.m_upper_bound;
    limit.inclusive_lower = r.m_inclusive_lower_bound;
    limit.inclusive_upper = r.m_inclusive_upper_bound;
    limit.negate = r.m_negate;
    return limit;
}

} // unnamed namespace

static PluginInfo const s_info = PluginInfo(
    "filters.radiusassign", "Re-assign some point attributes based KNN voting",
    "https://pdal.org/stages/filters.radiusassign.html");

CREATE_STATIC_STAGE(RadiusAssignFilter, s_info)

RadiusAssignFilter::RadiusAssignFilter() {}

RadiusAssignFilter::~RadiusAssignFilter() {}

void RadiusAssignFilter::addArgs(ProgramArgs& args)
{
    args.add("src_domain",
             "Selects which points will be subject to "
             "radius-based neighbors search",
             m_srcDomainSpec);
    args.add("reference_domain",
             "Selects which points will be considered as "
             "potential neighbors",
             m_referenceDomainSpec);
    args.add("radius", "Distance of neighbors to consult", m_radius);
    args.add("update_expression",
             "Value to assign to dimension of points of src_domain "
             "that have at least one neighbor in reference domain based on "
             "expression.",
             m_updateExpr);
    args.add("is3d", "Search in 3d", m_search3d, false);
    args.add("max2d_above",
             "if search in 2d : upward maximum distance in Z for potential "
             "neighbors "
             "(corresponds to a search in a cylinder with a height = "
             "max2d_above above the source point). "
             "Values < 0 mean infinite height",
             m_max2dAbove, -1.);
    args.add("max2d_below",
             "if search in 2d : downward maximum distance in Z for potential "
             "neighbors ("
             "corresponds to a search in a cylinder with a height = "
             "max2d_below below the source point). "
             "Values < 0 mean infinite height",
             m_max2dBelow, -1.);
}

void RadiusAssignFilter::initializeDomain(StringList domainSpec,
                                          std::vector<DimRange>& domain)
{
    for (auto const& r : domainSpec)
    {
        try
        {
            DimRange range;
            range.parse(r);
            domain.push_back(range);
        }
        catch (const DimRange::error& err)
        {
            throwError("Invalid 'domain' option: '" + r + "': " + err.what());
        }
    }
}

void RadiusAssignFilter::initialize()
{
    this->initializeDomain(m_referenceDomainSpec, m_referenceDomain);
    this->initializeDomain(m_srcDomainSpec, m_srcDomain);

    if (m_radius <= 0)
        throwError("Invalid 'radius' option: " + std::to_string(m_radius) +
                   ", must be > 0");

    if (m_updateExpr.empty())
        throwError("Empty 'update_expression' option, must be set to apply any "
                   "change on the data");
}

void RadiusAssignFilter::preparedDomain(std::vector<DimRange>& domain,
                                        PointLayoutPtr layout)
{
    for (auto& r : domain)
    {
        r.m_id = layout->findDim(r.m_name);
        if (r.m_id == Dimension::Id::Unknown)
            throwError("Invalid dimension name in 'srcDomain' option: '" +
                       r.m_name + "'.");
    }
    std::sort(domain.begin(), domain.end());
}

void RadiusAssignFilter::prepared(PointTableRef table)
{
    PointLayoutPtr layout(table.layout());
    this->preparedDomain(m_srcDomain, layout);
    this->preparedDomain(m_referenceDomain, layout);

    pdal_point_layout_t* rustLayout = rust_view_converter::toRustLayout(layout);
    for (const std::string& expr : m_updateExpr)
    {
        if (!pdal_stage_validate_assign_statement_with_layout(expr.c_str(),
                                                              rustLayout))
        {
            pdal_point_layout_destroy(rustLayout);
            rust_view_converter::throwLastError(
                "Invalid assignment expression in 'update_expression' "
                "option.");
        }
    }
    pdal_point_layout_destroy(rustLayout);
}

void RadiusAssignFilter::filter(PointView& view)
{
    std::vector<pdal_range_limit_t> srcLimits;
    for (const DimRange& r : m_srcDomain)
        srcLimits.push_back(toRustLimit(r));

    std::vector<pdal_range_limit_t> referenceLimits;
    for (const DimRange& r : m_referenceDomain)
        referenceLimits.push_back(toRustLimit(r));

    std::vector<const char*> assignmentPtrs;
    assignmentPtrs.reserve(m_updateExpr.size());
    for (const std::string& expression : m_updateExpr)
        assignmentPtrs.push_back(expression.c_str());

    pdal_point_view_t* rustView = rust_view_converter::toRust(view);
    pdal_stage_t* stage = pdal_stage_create_radiusassign_expr(
        srcLimits.empty() ? nullptr : srcLimits.data(), srcLimits.size(),
        referenceLimits.empty() ? nullptr : referenceLimits.data(),
        referenceLimits.size(),
        assignmentPtrs.empty() ? nullptr : assignmentPtrs.data(),
        assignmentPtrs.size(), m_radius, m_search3d, m_max2dAbove, m_max2dBelow,
        rustView);
    pdal_point_view_destroy(rustView);
    if (!stage)
        rust_view_converter::throwLastError(
            "Failed to create Rust radiusassign stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
