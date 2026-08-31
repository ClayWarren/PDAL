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

PointViewPtr makeGridWithOutlier(PointTable& table)
{
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

PointViewPtr run(PointTable& table, const Options& opts)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    BufferReader r;
    LOFFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    r.addView(makeGridWithOutlier(table));
    return *filter.execute(table).begin();
}

} // unnamed namespace

TEST(LOFFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.lof"));
    ASSERT_NE(filter, nullptr);
    LOFFilter l;
    EXPECT_EQ(l.getName(), "filters.lof");
}

TEST(LOFFilterTest, flags_outlier)
{
    PointTable table;
    PointViewPtr out = run(table, Options());
    ASSERT_EQ(out->size(), 26u);

    const double nnInlier =
        out->getFieldAs<double>(Dimension::Id::NNDistance, 12);
    const double nnOutlier =
        out->getFieldAs<double>(Dimension::Id::NNDistance, 25);
    const double lofInlier =
        out->getFieldAs<double>(Dimension::Id::LocalOutlierFactor, 12);
    const double lofOutlier =
        out->getFieldAs<double>(Dimension::Id::LocalOutlierFactor, 25);

    EXPECT_NEAR(nnInlier, 4.0, 1e-6);
    EXPECT_GT(
        out->getFieldAs<double>(Dimension::Id::LocalReachabilityDistance, 12),
        0.0);

    EXPECT_GT(nnOutlier, nnInlier);
    EXPECT_GT(nnOutlier, 100.0);

    EXPECT_GT(lofOutlier, lofInlier);
    EXPECT_GT(lofOutlier, 2.0);
    EXPECT_LT(lofInlier, 2.0);
}

TEST(LOFFilterTest, minpts_controls_k_distance)
{
    auto nnAt = [](size_t minpts)
    {
        PointTable table;
        Options opts;
        opts.add("minpts", minpts);
        PointViewPtr out = run(table, opts);
        EXPECT_EQ(out->size(), 26u);
        if (out->size() <= 12)
            return 0.0;
        return out->getFieldAs<double>(Dimension::Id::NNDistance, 12);
    };

    EXPECT_LT(nnAt(4), nnAt(10));
}
