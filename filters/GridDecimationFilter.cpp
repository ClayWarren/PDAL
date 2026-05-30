/******************************************************************************
 * Copyright (c) 2023, Antoine Lavenant (antoine.lavenant@ign.fr)
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

#include "GridDecimationFilter.hpp"

#include <pdal/PointView.hpp>
#include <pdal/StageFactory.hpp>

#include <pdal/private/gdal/GDALUtils.hpp>

#include "private/Point.hpp"

#include <cstdarg>
#include <sstream>
#include <pdal/private/RustViewConverter.hpp>
#include <pdal_capi.h>

extern "C" {
    uint64_t* pdal_grid_decimation_get_kept_indices(const pdal_point_view_t* view, double resolution, const char* output_type, uint64_t* out_len);
    void pdal_free_u64_array(uint64_t* ptr, uint64_t len);
    char* pdal_grid_decimation_validate(double resolution, const char* output_type);
}

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.gridDecimation", "keep max or min points in a grid",
    "https://pdal.org/stages/filters.GridDecimation.html"};

CREATE_STATIC_STAGE(GridDecimationFilter, s_info)

std::string GridDecimationFilter::getName() const
{
    return s_info.name;
}

GridDecimationFilter::GridDecimationFilter()
    : m_args(new GridDecimationFilter::GridArgs)
{
}

GridDecimationFilter::~GridDecimationFilter() {}

void GridDecimationFilter::addArgs(ProgramArgs& args)
{
    args.add("resolution", "Cell edge size, in units of X/Y",
             m_args->m_edgeLength, 1.);
    args.add("output_type", "Point keept into the cells ('min', 'max')",
             m_args->m_methodKeep, "max");
    args.add("value", "Value to assign to dimension based on expression.",
             m_args->m_statements);
}

void GridDecimationFilter::initialize() {}

void GridDecimationFilter::prepared(PointTableRef table)
{
    PointLayoutPtr layout(table.layout());

    for (expr::AssignStatement& expr : m_args->m_statements)
    {
        auto status = expr.prepare(layout);
        if (!status)
            throwError(status.what());
    }

    if (char* error = pdal_grid_decimation_validate(m_args->m_edgeLength,
                                                    m_args->m_methodKeep.c_str()))
    {
        std::string message(error);
        pdal_string_free(error);
        throwError(message);
    }
}

void GridDecimationFilter::ready(PointTableRef table)
{
    if (char* error = pdal_grid_decimation_validate(m_args->m_edgeLength,
                                                    m_args->m_methodKeep.c_str()))
    {
        std::string message(error);
        pdal_string_free(error);
        throwError(message);
    }
}

void GridDecimationFilter::processOne(BOX2D bounds, PointRef& point,
                                      PointViewPtr view)
{
}

void GridDecimationFilter::createGrid(BOX2D bounds)
{
}

PointViewSet GridDecimationFilter::run(PointViewPtr view)
{
    PointViewSet viewSet;

    uint64_t kept_len = 0;
    pdal_point_view_t* rust_in = rust_view_converter::toRust(view);
    uint64_t* kept_indices = pdal_grid_decimation_get_kept_indices(rust_in, m_args->m_edgeLength, m_args->m_methodKeep.c_str(), &kept_len);

    std::set<PointId> keepPoint;
    if (kept_indices)
    {
        for (uint64_t i = 0; i < kept_len; ++i)
        {
            keepPoint.insert(kept_indices[i]);
        }
        pdal_free_u64_array(kept_indices, kept_len);
    }
    pdal_point_view_destroy(rust_in);

    for (PointId i = 0; i < view->size(); ++i)
    {
        if (keepPoint.find(view->point(i).pointId()) != keepPoint.end())
        {
            PointRef point = view->point(i);
            for (expr::AssignStatement& expr : m_args->m_statements)
                if (expr.conditionalExpr().eval(point))
                    point.setField(expr.identExpr().eval(),
                                   expr.valueExpr().eval(point));
        }
    }

    viewSet.insert(view);
    return viewSet;
}

} // namespace pdal
