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

// Build a view from explicit coordinate triples, registering 'extraDim' first
// (when non-empty) so the layout is final before any point exists.
PointViewPtr makeView(PointTable& table,
                      const std::vector<std::array<double, 3>>& pts,
                      const std::string& extraDim = "")
{
    if (!extraDim.empty())
        table.layout()->registerOrAssignDim(extraDim,
                                            Dimension::Type::Unsigned8);
    table.layout()->registerDim(Dimension::Id::X);
    table.layout()->registerDim(Dimension::Id::Y);
    table.layout()->registerDim(Dimension::Id::Z);

    PointViewPtr view(new PointView(table));
    for (PointId i = 0; i < pts.size(); ++i)
    {
        view->setField(Dimension::Id::X, i, pts[i][0]);
        view->setField(Dimension::Id::Y, i, pts[i][1]);
        view->setField(Dimension::Id::Z, i, pts[i][2]);
    }
    return view;
}

PointViewPtr runSample(PointTable& table, PointViewPtr view,
                       const Options& opts)
{
    BufferReader r;
    r.addView(view);
    SampleFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
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

// With a 1.0 radius the second point of each tightly spaced pair is culled,
// leaving one representative per pair.
TEST(SampleFilterTest, culls_close_points)
{
    PointTable table;
    PointViewPtr view = makeView(table, {{{0.0, 0.0, 0.0}},
                                         {{0.1, 0.0, 0.0}},
                                         {{5.0, 0.0, 0.0}},
                                         {{5.1, 0.0, 0.0}},
                                         {{10.0, 0.0, 0.0}},
                                         {{10.1, 0.0, 0.0}}});

    Options opts;
    opts.add("radius", 1.0);
    PointViewPtr out = runSample(table, view, opts);

    ASSERT_EQ(out->size(), 3u);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 0), 0.0);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 1), 5.0);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 2), 10.0);
}

// Points farther apart than the radius are all retained.
TEST(SampleFilterTest, keeps_distant_points)
{
    PointTable table;
    PointViewPtr view = makeView(table, {{{0.0, 0.0, 0.0}},
                                         {{10.0, 0.0, 0.0}},
                                         {{20.0, 0.0, 0.0}},
                                         {{30.0, 0.0, 0.0}}});

    Options opts;
    opts.add("radius", 1.0);
    PointViewPtr out = runSample(table, view, opts);

    EXPECT_EQ(out->size(), 4u);
}

// 'cell' is an alternative to 'radius' (radius = cell/2 * sqrt(3)); setting
// both is an error, setting neither is an error.
TEST(SampleFilterTest, cell_mode)
{
    PointTable table;
    PointViewPtr view = makeView(table, {{{0.0, 0.0, 0.0}},
                                         {{0.1, 0.0, 0.0}},
                                         {{20.0, 0.0, 0.0}},
                                         {{20.1, 0.0, 0.0}}});

    Options opts;
    opts.add("cell", 4.0); // radius = 2 * sqrt(3) ~= 3.46
    PointViewPtr out = runSample(table, view, opts);

    // Each 0.1-spaced pair collapses to one point; the pairs are far apart.
    EXPECT_EQ(out->size(), 2u);

    PointTable bad;
    BufferReader r;
    r.addView(makeView(bad, {{{0.0, 0.0, 0.0}}}));
    SampleFilter both;
    both.setInput(r);
    Options bo;
    bo.add("cell", 1.0);
    bo.add("radius", 1.0);
    both.setOptions(bo);
    EXPECT_THROW(both.prepare(bad), pdal_error);
}

// A point in a voxel adjacent to an already-kept point must still be culled
// when it falls within the radius -- this exercises the neighbor-voxel search.
// Distinct non-zero coordinates make the squared-distance terms asymmetric.
TEST(SampleFilterTest, culls_across_voxels)
{
    PointTable table;
    // A and B land in adjacent voxels; dist(A,B)^2 = 0.64+0.04+0.16 = 0.84 < 1.
    // C is far from both.
    PointViewPtr view = makeView(
        table, {{{0.5, 0.5, 0.5}}, {{1.3, 0.7, 0.9}}, {{50.0, 50.0, 50.0}}});

    Options opts;
    opts.add("radius", 1.0);
    opts.add("origin_x", 0.0);
    opts.add("origin_y", 0.0);
    opts.add("origin_z", 0.0);
    PointViewPtr out = runSample(table, view, opts);

    ASSERT_EQ(out->size(), 2u);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 0), 0.5);
    EXPECT_DOUBLE_EQ(out->getFieldAs<double>(Dimension::Id::X, 1), 50.0);
}

// Just outside the radius the neighbor is kept; just inside it is culled.
TEST(SampleFilterTest, radius_boundary)
{
    auto countKept = [](double bx)
    {
        PointTable table;
        PointViewPtr view =
            makeView(table, {{{0.0, 0.0, 0.0}}, {{bx, 0.0, 0.0}}});
        Options opts;
        opts.add("radius", 1.0);
        opts.add("origin_x", 0.0);
        opts.add("origin_y", 0.0);
        opts.add("origin_z", 0.0);
        return runSample(table, view, opts)->size();
    };

    EXPECT_EQ(countKept(0.9), 1u); // within radius -> culled
    EXPECT_EQ(countKept(1.1), 2u); // beyond radius -> kept
}

// In 'dimension' mode no points are dropped; instead each point is flagged
// 1 (kept) or 0 (would have been culled).
TEST(SampleFilterTest, dimension_mode_flags_points)
{
    PointTable table;
    PointViewPtr view = makeView(table,
                                 {{{0.0, 0.0, 0.0}},
                                  {{0.1, 0.0, 0.0}},
                                  {{5.0, 0.0, 0.0}},
                                  {{5.1, 0.0, 0.0}},
                                  {{10.0, 0.0, 0.0}},
                                  {{10.1, 0.0, 0.0}}},
                                 "Sampled");

    Options opts;
    opts.add("radius", 1.0);
    opts.add("dimension", "Sampled");
    PointViewPtr out = runSample(table, view, opts);

    ASSERT_EQ(out->size(), 6u);
    Dimension::Id sampled = table.layout()->findDim("Sampled");
    ASSERT_NE(sampled, Dimension::Id::Unknown);

    const std::array<int, 6> expected = {1, 0, 1, 0, 1, 0};
    for (PointId i = 0; i < 6; ++i)
        EXPECT_EQ(out->getFieldAs<int>(sampled, i), expected[i]);
}

namespace
{

// A fixed 5x5x5 grid of points spaced 0.45 apart.
std::vector<std::array<double, 3>> denseGrid()
{
    std::vector<std::array<double, 3>> pts;
    for (int i = 0; i < 5; ++i)
        for (int j = 0; j < 5; ++j)
            for (int k = 0; k < 5; ++k)
                pts.push_back({{i * 0.45, j * 0.45, k * 0.45}});
    return pts;
}

} // unnamed namespace

// Characterization (radius mode): a dense grid sampled at radius 1.0 culls a
// fixed number of points. The explicit non-zero, distinct origin makes the
// per-axis voxel-index arithmetic ((coord - origin) / cell) decision-relevant,
// so any change to the voxelization, neighbor search, or distance arithmetic
// alters the survivor count and trips this test.
TEST(SampleFilterTest, characterization_radius)
{
    PointTable table;
    PointViewPtr view = makeView(table, denseGrid());

    Options opts;
    opts.add("radius", 1.0);
    opts.add("origin_x", 3.0);
    opts.add("origin_y", 7.0);
    opts.add("origin_z", 11.0);
    PointViewPtr out = runSample(table, view, opts);

    EXPECT_EQ(out->size(), 18u);
    // Sum the survivors' coordinates: this pins *which* points survive, not
    // just how many -- catching mutations that swap one survivor for another.
    double coordSum = 0.0;
    for (PointId i = 0; i < out->size(); ++i)
        coordSum += out->getFieldAs<double>(Dimension::Id::X, i) +
                    out->getFieldAs<double>(Dimension::Id::Y, i) +
                    out->getFieldAs<double>(Dimension::Id::Z, i);
    EXPECT_NEAR(coordSum, 48.15, 0.01);
}

// Characterization (cell mode): same grid, exercising the cell -> radius
// conversion path in ready() (radius = cell/2 * sqrt(3)).
TEST(SampleFilterTest, characterization_cell)
{
    PointTable table;
    PointViewPtr view = makeView(table, denseGrid());

    Options opts;
    opts.add("cell", 1.1547);
    opts.add("origin_x", 3.0);
    opts.add("origin_y", 7.0);
    opts.add("origin_z", 11.0);
    PointViewPtr out = runSample(table, view, opts);

    EXPECT_EQ(out->size(), 18u);
    double coordSum = 0.0;
    for (PointId i = 0; i < out->size(); ++i)
        coordSum += out->getFieldAs<double>(Dimension::Id::X, i) +
                    out->getFieldAs<double>(Dimension::Id::Y, i) +
                    out->getFieldAs<double>(Dimension::Id::Z, i);
    EXPECT_NEAR(coordSum, 48.15, 0.01);
}
