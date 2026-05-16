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

#include <filters/ExpressionStatsFilter.hpp>
#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/StageFactory.hpp>

using namespace pdal;

namespace
{

PointViewPtr makeView(PointTable& table)
{
    table.layout()->registerDim(Dimension::Id::X);
    PointViewPtr view(new PointView(table));
    const std::vector<double> xs = {1.0, 1.0, 2.0, 3.0, 3.0, 4.0};
    for (PointId i = 0; i < xs.size(); ++i)
        view->setField(Dimension::Id::X, i, xs[i]);
    return view;
}

MetadataNode findChild(MetadataNode node, const std::string& name)
{
    return node.findChild([&name](MetadataNode child)
                          { return child.name() == name; });
}

point_count_t countForValue(MetadataNode statistic, double value)
{
    for (MetadataNode bin : statistic.children("bins"))
    {
        MetadataNode valueNode = findChild(bin, "value");
        if (valueNode.empty() || valueNode.value<double>() != value)
            continue;
        return findChild(bin, "count").value<point_count_t>();
    }
    return 0;
}

} // unnamed namespace

TEST(ExpressionStatsFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.expressionstats"));
    EXPECT_TRUE(filter);
    ExpressionStatsFilter s;
    EXPECT_EQ(s.getName(), "filters.expressionstats");
}

TEST(ExpressionStatsFilterTest, metadata_bins_by_expression)
{
    PointTable table;
    BufferReader reader;
    reader.addView(makeView(table));

    ExpressionStatsFilter filter;
    filter.setInput(reader);
    Options opts;
    opts.add("dimension", "X");
    opts.add("expressions", "X < 3");
    opts.add("expressions", "X >= 3");
    filter.setOptions(opts);
    filter.prepare(table);
    filter.execute(table);

    MetadataNode metadata = filter.getMetadata();
    EXPECT_EQ(findChild(metadata, "dimension").value(), "X");

    std::vector<MetadataNode> stats = metadata.children("statistic");
    ASSERT_EQ(stats.size(), 2u);

    EXPECT_EQ(countForValue(stats[0], 1.0), 2u);
    EXPECT_EQ(countForValue(stats[0], 2.0), 1u);
    EXPECT_EQ(countForValue(stats[1], 3.0), 2u);
    EXPECT_EQ(countForValue(stats[1], 4.0), 1u);
}
