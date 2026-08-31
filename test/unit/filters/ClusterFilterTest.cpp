/******************************************************************************
 * Copyright (c) 2026, PDAL contributors
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
#include <set>
#include <vector>

#include <filters/ClusterFilter.hpp>
#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/StageFactory.hpp>

using namespace pdal;

namespace
{

PointViewPtr makeView(PointTable& table,
                      const std::vector<std::array<double, 3>>& pts)
{
    PointViewPtr view(new PointView(table));
    for (PointId i = 0; i < pts.size(); ++i)
    {
        view->setField(Dimension::Id::X, i, pts[i][0]);
        view->setField(Dimension::Id::Y, i, pts[i][1]);
        view->setField(Dimension::Id::Z, i, pts[i][2]);
    }
    return view;
}

PointViewPtr run(PointTable& table,
                 const std::vector<std::array<double, 3>>& pts,
                 const Options& opts)
{
    table.layout()->registerDim(Dimension::Id::X);
    table.layout()->registerDim(Dimension::Id::Y);
    table.layout()->registerDim(Dimension::Id::Z);

    BufferReader r;
    ClusterFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    r.addView(makeView(table, pts));
    return *filter.execute(table).begin();
}

} // unnamed namespace

TEST(ClusterFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.cluster"));
    ASSERT_NE(filter, nullptr);
    ClusterFilter c;
    EXPECT_EQ(c.getName(), "filters.cluster");
}

TEST(ClusterFilterTest, two_clusters)
{
    PointTable table;
    const std::array<std::array<double, 3>, 4> shape = {{{{0.0, 0.0, 0.0}},
                                                         {{0.5, 0.0, 0.0}},
                                                         {{0.0, 0.5, 0.0}},
                                                         {{0.5, 0.5, 0.0}}}};
    std::vector<std::array<double, 3>> pts;
    pts.reserve(8);
    for (auto& p : shape)
        pts.push_back(p);
    for (auto& p : shape)
        pts.push_back({{p[0] + 100.0, p[1], p[2]}});

    PointViewPtr out = run(table, pts, Options());
    ASSERT_EQ(out->size(), 8u);

    auto id = [&out](PointId i)
    { return out->getFieldAs<int>(Dimension::Id::ClusterID, i); };

    std::set<int> ids;
    for (PointId i = 0; i < 8; ++i)
        ids.insert(id(i));
    EXPECT_EQ(ids.size(), 2u);
    EXPECT_EQ(*ids.begin(), 1);
    EXPECT_EQ(*ids.rbegin(), 2);

    for (PointId i = 1; i < 4; ++i)
    {
        EXPECT_EQ(id(i), id(0));
        EXPECT_EQ(id(i + 4), id(4));
    }
    EXPECT_NE(id(0), id(4));
}

TEST(ClusterFilterTest, min_points_threshold)
{
    auto labeledCount = [](uint64_t minPoints)
    {
        PointTable table;
        std::vector<std::array<double, 3>> pts = {{{0.0, 0.0, 0.0}},
                                                  {{0.3, 0.0, 0.0}},
                                                  {{0.0, 0.3, 0.0}},
                                                  {{0.3, 0.3, 0.0}},
                                                  {{0.15, 0.15, 0.0}}};
        Options opts;
        opts.add("min_points", minPoints);
        PointViewPtr out = run(table, pts, opts);
        EXPECT_EQ(out->size(), 5u);
        size_t n = 0;
        for (PointId i = 0; i < out->size(); ++i)
            if (out->getFieldAs<int>(Dimension::Id::ClusterID, i) > 0)
                n++;
        return n;
    };

    EXPECT_EQ(labeledCount(3), 5u);
    EXPECT_EQ(labeledCount(100), 0u);
}

TEST(ClusterFilterTest, is3d_toggle)
{
    auto clusterCount = [](bool is3d)
    {
        PointTable table;
        std::vector<std::array<double, 3>> pts = {{{0.0, 0.0, 0.0}},
                                                  {{0.0, 0.0, 50.0}}};
        Options opts;
        opts.add("is3d", is3d);
        PointViewPtr out = run(table, pts, opts);
        EXPECT_EQ(out->size(), 2u);
        std::set<int> ids;
        for (PointId i = 0; i < out->size(); ++i)
            ids.insert(out->getFieldAs<int>(Dimension::Id::ClusterID, i));
        return ids.size();
    };

    EXPECT_EQ(clusterCount(false), 1u);
    EXPECT_EQ(clusterCount(true), 2u);
}
