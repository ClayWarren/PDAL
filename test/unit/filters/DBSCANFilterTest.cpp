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

#include <array>
#include <set>
#include <vector>

#include <filters/DBSCANFilter.hpp>
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
    DBSCANFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    r.addView(makeView(table, pts));
    return *filter.execute(table).begin();
}

std::vector<std::array<double, 3>> cube(double cx, double cy, double cz)
{
    std::vector<std::array<double, 3>> v;
    v.reserve(8);
    for (int i = 0; i < 8; ++i)
        v.push_back({{cx + ((i & 1) ? 0.3 : 0.0), cy + ((i & 2) ? 0.3 : 0.0),
                      cz + ((i & 4) ? 0.3 : 0.0)}});
    return v;
}

} // unnamed namespace

TEST(DBSCANFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.dbscan"));
    ASSERT_NE(filter, nullptr);
    DBSCANFilter d;
    EXPECT_EQ(d.getName(), "filters.dbscan");
}

TEST(DBSCANFilterTest, two_clusters_and_noise)
{
    PointTable table;
    std::vector<std::array<double, 3>> pts = cube(0.0, 0.0, 0.0);
    for (auto& p : cube(50.0, 50.0, 50.0))
        pts.push_back(p);
    pts.push_back({{500.0, 500.0, 500.0}});

    PointViewPtr out = run(table, pts, Options());
    ASSERT_EQ(out->size(), 17u);

    auto id = [&out](PointId i)
    { return out->getFieldAs<int>(Dimension::Id::ClusterID, i); };

    for (PointId i = 1; i < 8; ++i)
    {
        EXPECT_EQ(id(i), id(0));
        EXPECT_EQ(id(i + 8), id(8));
    }
    EXPECT_GE(id(0), 0);
    EXPECT_GE(id(8), 0);
    EXPECT_NE(id(0), id(8));
    EXPECT_EQ(id(16), -1);
}

TEST(DBSCANFilterTest, min_points_threshold)
{
    auto allNoise = [](uint64_t minPoints)
    {
        PointTable table;
        Options opts;
        opts.add("min_points", minPoints);
        opts.add("eps", 1.0);
        PointViewPtr out = run(table, cube(0.0, 0.0, 0.0), opts);
        EXPECT_EQ(out->size(), 8u);
        bool noise = true;
        for (PointId i = 0; i < out->size(); ++i)
            if (out->getFieldAs<int>(Dimension::Id::ClusterID, i) != -1)
                noise = false;
        return noise;
    };

    EXPECT_FALSE(allNoise(8));
    EXPECT_TRUE(allNoise(9));
}

TEST(DBSCANFilterTest, dimensions_restrict_clustering)
{
    auto clustered = [](const std::string& dims)
    {
        PointTable table;
        std::vector<std::array<double, 3>> pts;
        pts.reserve(6);
        for (int i = 0; i < 6; ++i)
            pts.push_back({{0.1 * (i & 1), 0.1 * (i & 1), i * 30.0}});
        Options opts;
        opts.add("min_points", static_cast<uint64_t>(6));
        opts.add("eps", 1.0);
        opts.add("dimensions", dims);
        PointViewPtr out = run(table, pts, opts);
        EXPECT_EQ(out->size(), 6u);
        for (PointId i = 0; i < out->size(); ++i)
            if (out->getFieldAs<int>(Dimension::Id::ClusterID, i) < 0)
                return false;
        return true;
    };

    EXPECT_TRUE(clustered("X, Y"));
    EXPECT_FALSE(clustered("X, Y, Z"));
}

TEST(DBSCANFilterTest, propagates_along_chain)
{
    PointTable table;
    std::vector<std::array<double, 3>> pts;
    pts.reserve(12);
    for (int i = 0; i < 12; ++i)
        pts.push_back({{i * 0.9, 0.0, 0.0}});

    Options opts;
    opts.add("eps", 1.0);
    opts.add("min_points", static_cast<uint64_t>(3));
    PointViewPtr out = run(table, pts, opts);
    ASSERT_EQ(out->size(), 12u);

    for (PointId i = 0; i < out->size(); ++i)
        EXPECT_EQ(out->getFieldAs<int>(Dimension::Id::ClusterID, i), 0);
}
