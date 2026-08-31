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

#include <random>

#include <filters/OptimalNeighborhoodFilter.hpp>
#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/StageFactory.hpp>

using namespace pdal;

namespace
{

PointViewPtr makeCloud(PointTable& table, PointId n)
{
    std::mt19937 gen(42);
    std::uniform_real_distribution<double> coord(0.0, 20.0);
    PointViewPtr view(new PointView(table));
    for (PointId i = 0; i < n; ++i)
    {
        view->setField(Dimension::Id::X, i, coord(gen));
        view->setField(Dimension::Id::Y, i, coord(gen));
        view->setField(Dimension::Id::Z, i, coord(gen));
    }
    return view;
}

PointViewPtr run(PointTable& table, PointId n, const Options& opts)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    BufferReader r;
    OptimalNeighborhood filter;
    filter.setInput(r);
    filter.setOptions(opts);
    filter.prepare(table);
    r.addView(makeCloud(table, n));
    return *filter.execute(table).begin();
}

} // unnamed namespace

TEST(OptimalNeighborhoodFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.optimalneighborhood"));
    ASSERT_NE(filter, nullptr);
    OptimalNeighborhood s;
    EXPECT_EQ(s.getName(), "filters.optimalneighborhood");
}

TEST(OptimalNeighborhoodFilterTest, k_within_bounds)
{
    PointTable table;

    Options opts;
    opts.add("min_k", 10);
    opts.add("max_k", 14);
    PointViewPtr out = run(table, 60, opts);
    ASSERT_EQ(out->size(), 60u);

    for (PointId i = 0; i < out->size(); ++i)
    {
        int k = out->getFieldAs<int>(Dimension::Id::OptimalKNN, i);
        EXPECT_GE(k, 10);
        EXPECT_LE(k, 14);
        EXPECT_GT(out->getFieldAs<double>(Dimension::Id::OptimalRadius, i),
                  0.0);
    }
}

TEST(OptimalNeighborhoodFilterTest, custom_k_window)
{
    PointTable table;

    Options opts;
    opts.add("min_k", 5);
    opts.add("max_k", 8);
    PointViewPtr out = run(table, 60, opts);
    ASSERT_EQ(out->size(), 60u);

    bool sawBelowDefault = false;
    for (PointId i = 0; i < out->size(); ++i)
    {
        int k = out->getFieldAs<int>(Dimension::Id::OptimalKNN, i);
        EXPECT_GE(k, 5);
        EXPECT_LE(k, 8);
        if (k < 10)
            sawBelowDefault = true;
    }
    EXPECT_TRUE(sawBelowDefault);
}
