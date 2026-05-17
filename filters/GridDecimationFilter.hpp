/******************************************************************************
 * Copyright (c) 2023, Antoine Lavenant (antoine.lavenant@ign.fr)
 *
 * All rights reserved.
 *
 ****************************************************************************/

#pragma once

#include <list>
#include <memory>

#include <pdal/Filter.hpp>
#include <pdal/PointRef.hpp>
#include <pdal/Polygon.hpp>

#include "private/expr/AssignStatement.hpp"

namespace pdal
{

// keep selected points on a grid
class PDAL_EXPORT GridDecimationFilter : public Filter
{
public:
    GridDecimationFilter();
    ~GridDecimationFilter() override;

    std::string getName() const override;

private:
    struct GridArgs
    {
        std::string m_methodKeep; // type of output (min, max)
        double m_edgeLength;      // lenght of grid
        std::vector<expr::AssignStatement> m_statements;
    };

    std::unique_ptr<GridArgs> m_args;

    typedef std::pair<int, int> coordsGrid;
    std::map<coordsGrid, long> grid;

    void addArgs(ProgramArgs& args) override;
    void initialize() override;

    void ready(PointTableRef table) override;
    PointViewSet run(PointViewPtr view) override;
    void prepared(PointTableRef table) override;

    void createGrid(BOX2D bounds);
    void processOne(BOX2D bounds, PointRef& point, PointViewPtr view);

    GridDecimationFilter&
    operator=(const GridDecimationFilter&);            // not implemented
    GridDecimationFilter(const GridDecimationFilter&); // not implemented
};

} // namespace pdal
