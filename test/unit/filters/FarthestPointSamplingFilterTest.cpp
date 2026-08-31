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

#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/Stage.hpp>
#include <pdal/StageFactory.hpp>

#include <algorithm>
#include <vector>

using namespace pdal;

namespace
{

PointViewPtr makeCloud(PointTable& table, PointId count)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    PointViewPtr view(new PointView(table));
    for (PointId i = 0; i < count; ++i)
    {
        view->setField(Dimension::Id::X, i, static_cast<double>(i));
        view->setField(Dimension::Id::Y, i, 0.0);
        view->setField(Dimension::Id::Z, i, 0.0);
    }
    return view;
}

PointViewPtr run(PointTable& table, PointId inputCount, const Options& opts)
{
    BufferReader reader;
    StageFactory f;
    Stage* filter(f.createStage("filters.fps"));
    filter->setInput(reader);
    filter->setOptions(opts);
    filter->prepare(table);
    reader.addView(makeCloud(table, inputCount));
    return *filter->execute(table).begin();
}

} // unnamed namespace

TEST(FarthestPointSamplingFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.fps"));
    ASSERT_NE(filter, nullptr);
    EXPECT_EQ(filter->getName(), "filters.fps");
}

TEST(FarthestPointSamplingFilterTest, samples_to_count)
{
    PointTable table;
    Options opts;
    opts.add("count", 10);
    PointViewPtr out = run(table, 50, opts);

    ASSERT_EQ(out->size(), 10u);

    std::vector<double> xs;
    for (PointId i = 0; i < out->size(); ++i)
        xs.push_back(out->getFieldAs<double>(Dimension::Id::X, i));
    const auto range = std::minmax_element(xs.begin(), xs.end());
    EXPECT_DOUBLE_EQ(*range.first, 0.0);
    EXPECT_DOUBLE_EQ(*range.second, 49.0);
}

TEST(FarthestPointSamplingFilterTest, fewer_points_than_count)
{
    PointTable table;
    Options opts;
    opts.add("count", 20);
    PointViewPtr out = run(table, 5, opts);

    EXPECT_EQ(out->size(), 5u);
}
