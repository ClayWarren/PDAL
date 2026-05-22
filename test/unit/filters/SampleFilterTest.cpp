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
#include <vector>

#include <filters/SampleFilter.hpp>
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

PointViewPtr runSample(PointTable& table,
                       const std::vector<std::array<double, 3>>& pts,
                       const Options& opts)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    BufferReader r;
    SampleFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    r.addView(makeView(table, pts));
    return *filter.execute(table).begin();
}

} // unnamed namespace

TEST(SampleFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.sample"));
    EXPECT_TRUE(filter);
    SampleFilter s;
    EXPECT_EQ(s.getName(), "filters.sample");
}

TEST(SampleFilterTest, culls_close_points)
{
    PointTable table;
    std::vector<std::array<double, 3>> pts = {
        {{0.0, 0.0, 0.0}}, {{0.1, 0.0, 0.0}},  {{5.0, 0.0, 0.0}},
        {{5.1, 0.0, 0.0}}, {{10.0, 0.0, 0.0}}, {{10.1, 0.0, 0.0}}};

    Options opts;
    opts.add("radius", 1.0);
    PointViewPtr out = runSample(table, pts, opts);

    ASSERT_EQ(out->size(), 3u);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 0), 0.0);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 1), 5.0);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 2), 10.0);
}

TEST(SampleFilterTest, keeps_distant_points)
{
    PointTable table;
    std::vector<std::array<double, 3>> pts = {{{0.0, 0.0, 0.0}},
                                              {{10.0, 0.0, 0.0}},
                                              {{20.0, 0.0, 0.0}},
                                              {{30.0, 0.0, 0.0}}};

    Options opts;
    opts.add("radius", 1.0);
    PointViewPtr out = runSample(table, pts, opts);

    EXPECT_EQ(out->size(), 4u);
}

TEST(SampleFilterTest, cell_mode)
{
    PointTable table;
    std::vector<std::array<double, 3>> pts = {{{0.0, 0.0, 0.0}},
                                              {{0.1, 0.0, 0.0}},
                                              {{20.0, 0.0, 0.0}},
                                              {{20.1, 0.0, 0.0}}};

    Options opts;
    opts.add("cell", 4.0);
    PointViewPtr out = runSample(table, pts, opts);

    EXPECT_EQ(out->size(), 2u);
}

TEST(SampleFilterTest, requires_cell_or_radius)
{
    PointTable bad;
    bad.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    BufferReader r;
    SampleFilter neither;
    neither.setInput(r);
    neither.setOptions(Options());
    EXPECT_THROW(neither.prepare(bad), pdal_error);
}

TEST(SampleFilterTest, rejects_cell_and_radius)
{
    PointTable bad;
    bad.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    BufferReader r;
    SampleFilter both;
    both.setInput(r);
    Options bo;
    bo.add("cell", 1.0);
    bo.add("radius", 1.0);
    both.setOptions(bo);
    EXPECT_THROW(both.prepare(bad), pdal_error);
}

TEST(SampleFilterTest, culls_across_voxels)
{
    PointTable table;
    std::vector<std::array<double, 3>> pts = {
        {{0.5, 0.5, 0.5}}, {{1.3, 0.7, 0.9}}, {{50.0, 50.0, 50.0}}};

    Options opts;
    opts.add("radius", 1.0);
    opts.add("origin_x", 0.0);
    opts.add("origin_y", 0.0);
    opts.add("origin_z", 0.0);
    PointViewPtr out = runSample(table, pts, opts);

    ASSERT_EQ(out->size(), 2u);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 0), 0.5);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 1), 50.0);
}

TEST(SampleFilterTest, radius_boundary)
{
    auto countKept = [](double bx, double originX)
    {
        PointTable table;
        std::vector<std::array<double, 3>> pts = {{{0.0, 0.0, 0.0}},
                                                  {{bx, 0.0, 0.0}}};
        Options opts;
        opts.add("radius", 1.0);
        opts.add("origin_x", originX);
        opts.add("origin_y", 0.0);
        opts.add("origin_z", 0.0);
        return runSample(table, pts, opts)->size();
    };

    EXPECT_EQ(countKept(0.9, 0.0), 1u);
    EXPECT_EQ(countKept(1.1, 0.0), 2u);

    EXPECT_EQ(countKept(1.0, 0.0), 2u);
    EXPECT_EQ(countKept(1.0, -0.6), 2u);
}

TEST(SampleFilterTest, repeated_execute_resets_voxels)
{
    PointTable table;
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    PointViewPtr view =
        makeView(table, {{{0.0, 0.0, 0.0}}, {{10.0, 0.0, 0.0}}});

    BufferReader r;
    r.addView(view);

    SampleFilter filter;
    filter.setInput(r);
    Options opts;
    opts.add("radius", 1.0);
    filter.setOptions(opts);
    filter.prepare(table);

    PointViewPtr first = *filter.execute(table).begin();
    PointViewPtr second = *filter.execute(table).begin();

    EXPECT_EQ(first->size(), 2u);
    EXPECT_EQ(second->size(), 2u);
}

TEST(SampleFilterTest, dimension_mode_flags_points)
{
    PointTable table;
    std::vector<std::array<double, 3>> pts = {
        {{0.0, 0.0, 0.0}}, {{0.1, 0.0, 0.0}},  {{5.0, 0.0, 0.0}},
        {{5.1, 0.0, 0.0}}, {{10.0, 0.0, 0.0}}, {{10.1, 0.0, 0.0}}};

    Options opts;
    opts.add("radius", 1.0);
    opts.add("dimension", "Sampled");
    PointViewPtr out = runSample(table, pts, opts);

    ASSERT_EQ(out->size(), 6u);
    Dimension::Id sampled = table.layout()->findDim("Sampled");
    ASSERT_NE(sampled, Dimension::Id::Unknown);

    const std::array<int, 6> expected = {1, 0, 1, 0, 1, 0};
    for (PointId i = 0; i < 6; ++i)
        EXPECT_EQ(out->getFieldAs<int>(sampled, i), expected[i]);
}

namespace
{

std::vector<std::array<double, 3>> denseGrid()
{
    std::vector<std::array<double, 3>> pts;
    for (int i = 0; i < 5; ++i)
        for (int j = 0; j < 5; ++j)
            for (int k = 0; k < 5; ++k)
                pts.push_back({{i * 0.45, j * 0.45, k * 0.45}});
    return pts;
}

void expectMinDistance(const PointViewPtr& view, double minDistance)
{
    const double minSqr = minDistance * minDistance;
    for (PointId i = 0; i < view->size(); ++i)
        for (PointId j = i + 1; j < view->size(); ++j)
        {
            const double dx = view->getFieldAs<double>(Dimension::Id::X, i) -
                              view->getFieldAs<double>(Dimension::Id::X, j);
            const double dy = view->getFieldAs<double>(Dimension::Id::Y, i) -
                              view->getFieldAs<double>(Dimension::Id::Y, j);
            const double dz = view->getFieldAs<double>(Dimension::Id::Z, i) -
                              view->getFieldAs<double>(Dimension::Id::Z, j);
            EXPECT_GE(dx * dx + dy * dy + dz * dz, minSqr);
        }
}

} // unnamed namespace

TEST(SampleFilterTest, dense_grid_radius_spacing)
{
    PointTable table;

    Options opts;
    opts.add("radius", 1.0);
    opts.add("origin_x", 3.0);
    opts.add("origin_y", 7.0);
    opts.add("origin_z", 11.0);
    PointViewPtr out = runSample(table, denseGrid(), opts);

    EXPECT_EQ(out->size(), 18u);
    expectMinDistance(out, 1.0);
}

TEST(SampleFilterTest, dense_grid_cell_spacing)
{
    PointTable table;

    Options opts;
    opts.add("cell", 1.1547);
    opts.add("origin_x", 3.0);
    opts.add("origin_y", 7.0);
    opts.add("origin_z", 11.0);
    PointViewPtr out = runSample(table, denseGrid(), opts);

    EXPECT_EQ(out->size(), 18u);
    expectMinDistance(out, 1.0);
}
