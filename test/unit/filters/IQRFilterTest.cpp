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

#include <filters/IQRFilter.hpp>
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

PointViewPtr run(PointTable& table, const PointViewPtr& view,
                 const Options& opts)
{
    BufferReader r;
    r.addView(view);
    IQRFilter filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    return *filter.execute(table).begin();
}

Options dimX(double k = -1.0)
{
    Options o;
    o.add("dimension", "X");
    if (k >= 0.0)
        o.add("k", k);
    return o;
}

} // unnamed namespace

TEST(IQRFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.iqr"));
    ASSERT_NE(filter, nullptr);
    IQRFilter i;
    EXPECT_EQ(i.getName(), "filters.iqr");
}

TEST(IQRFilterTest, drops_high_outlier)
{
    PointTable table;
    PointViewPtr view =
        makeView(table, {1,  2,  3,  4,  5,  6,  7,  8,  9,  10,  11,
                         12, 13, 14, 15, 16, 17, 18, 19, 20, 1000});

    PointViewPtr out = run(table, view, dimX());
    ASSERT_EQ(out->size(), 20u);
    for (PointId i = 0; i < out->size(); ++i)
        EXPECT_LE(out->getFieldAs<double>(Dimension::Id::X, i), 20.0);
}

TEST(IQRFilterTest, drops_low_outlier)
{
    PointTable table;
    PointViewPtr view =
        makeView(table, {-1000, 1,  2,  3,  4,  5,  6,  7,  8,  9, 10,
                         11,    12, 13, 14, 15, 16, 17, 18, 19, 20});

    PointViewPtr out = run(table, view, dimX());
    ASSERT_EQ(out->size(), 20u);
    for (PointId i = 0; i < out->size(); ++i)
        EXPECT_GE(out->getFieldAs<double>(Dimension::Id::X, i), 1.0);
}

TEST(IQRFilterTest, fence_boundary_is_inclusive)
{
    auto sizeWithLast = [](double last)
    {
        PointTable table;
        std::vector<double> xs;
        for (int v = 1; v <= 20; ++v)
            xs.push_back(v);
        xs.push_back(last);
        return run(table, makeView(table, xs), dimX())->size();
    };

    EXPECT_EQ(sizeWithLast(30.0), 21u);
    EXPECT_EQ(sizeWithLast(31.0), 21u);
}

TEST(IQRFilterTest, multiplier_widens_fence)
{
    auto runWith = [](double k)
    {
        PointTable table;
        std::vector<double> xs;
        for (int v = 1; v <= 20; ++v)
            xs.push_back(v);
        xs.push_back(60.0);
        return run(table, makeView(table, xs), dimX(k))->size();
    };

    EXPECT_LT(runWith(1.5), runWith(20.0));
    EXPECT_EQ(runWith(20.0), 21u);
}

TEST(IQRFilterTest, missing_dimension_throws)
{
    PointTable table;
    BufferReader r;
    r.addView(makeView(table, {1, 2, 3}));
    IQRFilter filter;
    filter.setInput(r);
    Options opts;
    opts.add("dimension", "NoSuchDim");
    filter.setOptions(opts);
    EXPECT_THROW(filter.prepare(table), pdal_error);
}
