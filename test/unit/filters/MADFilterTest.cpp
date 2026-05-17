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

#include <vector>

#include <filters/MADFilter.hpp>
#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/StageFactory.hpp>

using namespace pdal;

namespace
{

PointViewPtr makeView(PointTable& table, const std::vector<double>& xs)
{
    table.layout()->registerDim(Dimension::Id::X);
    PointViewPtr view(new PointView(table));
    for (PointId i = 0; i < xs.size(); ++i)
        view->setField(Dimension::Id::X, i, xs[i]);
    return view;
}

point_count_t runSize(PointTable& table, const PointViewPtr& view,
                      const Options& opts)
{
    BufferReader r;
    r.addView(view);
    MADFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    return (*filter.execute(table).begin())->size();
}

} // unnamed namespace

TEST(MADFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.mad"));
    EXPECT_TRUE(filter);
    MADFilter m;
    EXPECT_EQ(m.getName(), "filters.mad");
}

TEST(MADFilterTest, drops_outlier)
{
    PointTable table;
    PointViewPtr view = makeView(table, {1, 2, 3, 4, 5, 6, 7, 8, 9, 1000});

    Options opts;
    opts.add("dimension", "X");

    BufferReader r;
    r.addView(view);
    MADFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    PointViewPtr out = *filter.execute(table).begin();

    ASSERT_EQ(out->size(), 9u);
    for (PointId i = 0; i < out->size(); ++i)
        EXPECT_LE(out->getFieldAs<double>(Dimension::Id::X, i), 9.0);
}

TEST(MADFilterTest, keeps_clean_data)
{
    PointTable table;
    PointViewPtr view =
        makeView(table, {1,  2,  3,  4,  5,  6,  7,  8,  9,  10,
                         11, 12, 13, 14, 15, 16, 17, 18, 19, 20});

    Options opts;
    opts.add("dimension", "X");
    EXPECT_EQ(runSize(table, view, opts), 20u);
}

TEST(MADFilterTest, missing_dimension_throws)
{
    PointTable table;
    BufferReader r;
    r.addView(makeView(table, {1, 2, 3}));
    MADFilter filter;
    filter.setInput(r);
    Options opts;
    opts.add("dimension", "NoSuchDim");
    filter.setOptions(opts);
    EXPECT_THROW(filter.prepare(table), pdal_error);
}

TEST(MADFilterTest, parameters_affect_fence)
{
    const std::vector<double> data = {1,  2,  3,  4,  5,  6,  7,  8,  9,  10,
                                      11, 12, 13, 14, 15, 16, 17, 18, 19, 40};

    auto sizeWith = [&data](double k, double madMult)
    {
        PointTable table;
        PointViewPtr view = makeView(table, data);
        Options opts;
        opts.add("dimension", "X");
        opts.add("k", k);
        opts.add("mad_multiplier", madMult);
        return runSize(table, view, opts);
    };

    EXPECT_LE(sizeWith(1.0, 1.4862), sizeWith(8.0, 1.4862));
    EXPECT_LT(sizeWith(1.0, 1.4862), 20u);
    EXPECT_EQ(sizeWith(50.0, 1.4862), 20u);
}
