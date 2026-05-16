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

#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/Stage.hpp>
#include <pdal/StageFactory.hpp>

using namespace pdal;

namespace
{

using Pt = std::array<double, 3>;

PointViewPtr run(PointTable& table, const std::vector<Pt>& pts,
                 const Options& opts)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    BufferReader reader;
    StageFactory f;
    Stage* filter(f.createStage("filters.straighten"));
    filter->setInput(reader);
    filter->setOptions(opts);
    filter->prepare(table);

    PointViewPtr view(new PointView(table));
    for (PointId i = 0; i < pts.size(); ++i)
    {
        view->setField(Dimension::Id::X, i, pts[i][0]);
        view->setField(Dimension::Id::Y, i, pts[i][1]);
        view->setField(Dimension::Id::Z, i, pts[i][2]);
    }
    reader.addView(view);
    return *filter->execute(table).begin();
}

std::vector<Pt> coords(const PointViewPtr& v)
{
    std::vector<Pt> out;
    out.reserve(v->size());
    for (PointId i = 0; i < v->size(); ++i)
        out.push_back({{v->getFieldAs<double>(Dimension::Id::X, i),
                        v->getFieldAs<double>(Dimension::Id::Y, i),
                        v->getFieldAs<double>(Dimension::Id::Z, i)}});
    return out;
}

} // unnamed namespace

TEST(StraightenFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.straighten"));
    EXPECT_TRUE(filter);
    EXPECT_EQ(filter->getName(), "filters.straighten");
}

TEST(StraightenFilterTest, invalid_polyline_throws)
{
    PointTable table;
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    BufferReader reader;
    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 0.0);
    reader.addView(view);

    StageFactory f;
    Stage* filter(f.createStage("filters.straighten"));
    filter->setInput(reader);
    Options opts;
    opts.add("polyline", "not a polyline");
    filter->setOptions(opts);
    EXPECT_THROW(filter->prepare(table), pdal_error);
}

TEST(StraightenFilterTest, straighten_then_unstraighten_round_trips)
{
    const std::string poly = "LINESTRING ZM (0 0 0 0, 0 100 0 0)";
    const std::vector<Pt> orig = {
        {{2.0, 25.0, 1.0}}, {{-1.0, 50.0, 0.5}}, {{3.0, 75.0, 2.0}}};

    PointTable t1;
    Options straightenOpts;
    straightenOpts.add("polyline", poly);
    std::vector<Pt> straightened = coords(run(t1, orig, straightenOpts));
    ASSERT_EQ(straightened.size(), orig.size());

    EXPECT_GT(straightened[0][0], 10.0);

    PointTable t2;
    Options reverseOpts;
    reverseOpts.add("polyline", poly);
    reverseOpts.add("reverse", true);
    std::vector<Pt> back = coords(run(t2, straightened, reverseOpts));

    ASSERT_EQ(back.size(), orig.size());
    for (size_t i = 0; i < orig.size(); ++i)
    {
        EXPECT_NEAR(back[i][0], orig[i][0], 1e-6);
        EXPECT_NEAR(back[i][1], orig[i][1], 1e-6);
        EXPECT_NEAR(back[i][2], orig[i][2], 1e-6);
    }
}
