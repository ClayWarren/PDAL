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

#include <filters/LOFFilter.hpp>
#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/StageFactory.hpp>

using namespace pdal;

namespace
{

// A uniform 5x5 grid (spacing 2, z=0) plus one far-flung outlier at id 25.
PointViewPtr makeGridWithOutlier(PointTable& table)
{
    table.layout()->registerDim(Dimension::Id::X);
    table.layout()->registerDim(Dimension::Id::Y);
    table.layout()->registerDim(Dimension::Id::Z);
    table.layout()->registerDim(Dimension::Id::NNDistance);
    table.layout()->registerDim(Dimension::Id::LocalReachabilityDistance);
    table.layout()->registerDim(Dimension::Id::LocalOutlierFactor);

    PointViewPtr view(new PointView(table));
    PointId idx = 0;
    for (int i = 0; i < 5; ++i)
        for (int j = 0; j < 5; ++j, ++idx)
        {
            view->setField(Dimension::Id::X, idx, i * 2.0);
            view->setField(Dimension::Id::Y, idx, j * 2.0);
            view->setField(Dimension::Id::Z, idx, 0.0);
        }
    view->setField(Dimension::Id::X, 25, 1000.0);
    view->setField(Dimension::Id::Y, 25, 1000.0);
    view->setField(Dimension::Id::Z, 25, 1000.0);
    return view;
}

PointViewPtr run(PointTable& table, PointViewPtr view, const Options& opts)
{
    BufferReader r;
    r.addView(view);
    LOFFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    return *filter.execute(table).begin();
}

} // unnamed namespace

TEST(LOFFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.lof"));
    EXPECT_TRUE(filter);
    LOFFilter l;
    EXPECT_EQ(l.getName(), "filters.lof");
}

// The outlier scores a markedly higher local outlier factor and neighbor
// distance than a point in the dense interior of the grid.
TEST(LOFFilterTest, flags_outlier)
{
    PointTable table;
    PointViewPtr out = run(table, makeGridWithOutlier(table), Options());
    ASSERT_EQ(out->size(), 26u);

    // Point id 12 is the center of the 5x5 grid; 25 is the outlier.
    const double nnInlier =
        out->getFieldAs<double>(Dimension::Id::NNDistance, 12);
    const double nnOutlier =
        out->getFieldAs<double>(Dimension::Id::NNDistance, 25);
    const double lofInlier =
        out->getFieldAs<double>(Dimension::Id::LocalOutlierFactor, 12);
    const double lofOutlier =
        out->getFieldAs<double>(Dimension::Id::LocalOutlierFactor, 25);

    // NNDistance is the distance to the (minpts+1)-th neighbor. On this grid
    // (spacing 2) the 11th-nearest neighbor of the center point lies at
    // offset (4,0) from it -- a distance of exactly 4.0.
    EXPECT_NEAR(nnInlier, 4.0, 1e-6);
    EXPECT_GT(
        out->getFieldAs<double>(Dimension::Id::LocalReachabilityDistance, 12),
        0.0);

    // The outlier sits much farther from its neighbors.
    EXPECT_GT(nnOutlier, nnInlier);
    EXPECT_GT(nnOutlier, 100.0);

    // And is scored as a far stronger outlier; a grid interior point is not.
    EXPECT_GT(lofOutlier, lofInlier);
    EXPECT_GT(lofOutlier, 2.0);
    EXPECT_LT(lofInlier, 2.0);
}

// 'minpts' selects which neighbor sets the k-distance; a smaller value yields
// a nearer neighbor and thus a smaller NNDistance.
TEST(LOFFilterTest, minpts_controls_k_distance)
{
    auto nnAt = [](size_t minpts)
    {
        PointTable table;
        Options opts;
        opts.add("minpts", minpts);
        PointViewPtr out = run(table, makeGridWithOutlier(table), opts);
        return out->getFieldAs<double>(Dimension::Id::NNDistance, 12);
    };

    EXPECT_LT(nnAt(4), nnAt(10));
}

// Characterization: the three-pass reachability/LOF arithmetic is hard to
// derive by hand, so pin the aggregate for the fixed grid+outlier cloud. Any
// change to the reachability-distance or LOF math shifts these sums.
TEST(LOFFilterTest, characterization)
{
    PointTable table;
    PointViewPtr out = run(table, makeGridWithOutlier(table), Options());
    ASSERT_EQ(out->size(), 26u);

    double nnSum = 0.0;
    double lofSum = 0.0;
    for (PointId i = 0; i < out->size(); ++i)
    {
        nnSum += out->getFieldAs<double>(Dimension::Id::NNDistance, i);
        lofSum += out->getFieldAs<double>(Dimension::Id::LocalOutlierFactor, i);
    }
    // Pinned for the fixed grid+outlier cloud.
    EXPECT_NEAR(nnSum, 1839.95, 1.0);
    EXPECT_NEAR(lofSum, 376.933, 0.5);
}
