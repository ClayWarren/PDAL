/******************************************************************************
 * Copyright (c) 2026, Hobu Inc.
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

#include <pdal/pdal_test_main.hpp>

#include <algorithm>
#include <array>
#include <numeric>
#include <vector>

#include <pdal/PointView.hpp>
#include <pdal/QuadIndex.hpp>

namespace pdal
{

namespace
{

PointViewPtr makeView(PointTable& table,
                      const std::vector<std::array<double, 2>>& pts)
{
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y});
    PointViewPtr view(new PointView(table));
    for (PointId i = 0; i < pts.size(); ++i)
    {
        view->setField(Dimension::Id::X, i, pts[i][0]);
        view->setField(Dimension::Id::Y, i, pts[i][1]);
    }
    return view;
}

std::vector<PointId> sorted(PointIdList ids)
{
    std::sort(ids.begin(), ids.end());
    return ids;
}

} // unnamed namespace

TEST(QuadIndexTest, bounds_and_region_queries)
{
    PointTable table;
    PointViewPtr view = makeView(
        table,
        {{{0.0, 0.0}}, {{9.0, 0.0}}, {{0.0, 9.0}}, {{9.0, 9.0}}, {{4.0, 4.0}}});

    QuadIndex index(*view);

    double minx;
    double miny;
    double maxx;
    double maxy;
    index.getBounds(minx, miny, maxx, maxy);
    EXPECT_DOUBLE_EQ(minx, 0.0);
    EXPECT_DOUBLE_EQ(miny, 0.0);
    EXPECT_DOUBLE_EQ(maxx, 9.0);
    EXPECT_DOUBLE_EQ(maxy, 9.0);

    EXPECT_EQ(sorted(index.getPoints()), (std::vector<PointId>{0, 1, 2, 3, 4}));
    EXPECT_EQ(sorted(index.getPoints(0.0, 0.0, 5.0, 5.0)),
              (std::vector<PointId>{0, 4}));
    EXPECT_EQ(sorted(index.getPoints(5.0, 5.0, 10.0, 10.0)),
              (std::vector<PointId>{3}));

    std::vector<std::size_t> fills = index.getFills();
    ASSERT_EQ(fills.size(), index.getDepth() + 1);
    EXPECT_EQ(std::accumulate(fills.begin(), fills.end(), std::size_t(0)),
              view->size());
}

} // namespace pdal
