#pragma once

#include <pdal/Filter.hpp>

extern "C" int32_t RadiusAssignFilter_ExitFunc();
extern "C" PF_ExitFunc RadiusAssignFilter_InitPlugin();

namespace pdal
{

struct DimRange;

class PDAL_EXPORT RadiusAssignFilter : public Filter
{
public:
    RadiusAssignFilter();
    ~RadiusAssignFilter() override;

    static void* create();
    static int32_t destroy(void*);
    std::string getName() const override
    {
        return "filters.radiusassign";
    }

private:
    void addArgs(ProgramArgs& args) override;
    virtual void preparedDomain(std::vector<DimRange>& domain,
                                PointLayoutPtr layout);
    void prepared(PointTableRef table) override;
    void filter(PointView& view) override;
    virtual void initializeDomain(StringList domainSpec,
                                  std::vector<DimRange>& domain);
    void initialize() override;
    RadiusAssignFilter& operator=(const RadiusAssignFilter&) = delete;
    RadiusAssignFilter(const RadiusAssignFilter&) = delete;
    StringList m_referenceDomainSpec;
    std::vector<DimRange> m_referenceDomain;
    StringList m_srcDomainSpec;
    std::vector<DimRange> m_srcDomain;
    double m_radius;
    StringList m_updateExpr;
    bool m_search3d;
    double m_max2dAbove;
    double m_max2dBelow;
};

} // namespace pdal
