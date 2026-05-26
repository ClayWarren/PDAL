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

PointViewPtr run(PointTable& table,
                 const std::vector<std::array<double, 3>>& pts,
                 const Options& opts)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    BufferReader reader;
    StageFactory f;
    Stage* filter(f.createStage("filters.zsmooth"));
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

} // unnamed namespace

TEST(ZsmoothFilterTest, create)
{
    StageFactory f;
    Stage* filter = f.createStage("filters.zsmooth");
    EXPECT_TRUE(filter);

    Options opts;
    opts.add("radius", 1.0);
    opts.add("dim", "Zsmooth");
    filter->setOptions(opts);

    BufferReader reader;
    PointTable table;
    reader.addView(PointViewPtr(new PointView(table)));
    filter->setInput(reader);
    filter->prepare(table);

    EXPECT_EQ(filter->getName(), "filters.zsmooth");
}

TEST(ZsmoothFilterTest, medianpercent_selects_neighbor_z)
{
    std::vector<std::array<double, 3>> pts = {{{0.0, 0.0, 5.0}},
                                              {{0.1, 0.0, 10.0}},
                                              {{0.1, 0.0, 20.0}},
                                              {{0.1, 0.0, 30.0}},
                                              {{0.1, 0.0, 40.0}}};

    auto smoothed = [&pts](double percent)
    {
        PointTable table;
        Options opts;
        opts.add("dim", "Zsmoothed");
        opts.add("radius", 1.0);
        opts.add("medianpercent", percent);
        PointViewPtr out = run(table, pts, opts);
        Dimension::Id d = table.layout()->findDim("Zsmoothed");
        return out->getFieldAs<double>(d, 0);
    };

    EXPECT_DOUBLE_EQ(smoothed(0.0), 10.0);
    EXPECT_DOUBLE_EQ(smoothed(50.0), 25.0);
    EXPECT_DOUBLE_EQ(smoothed(100.0), 40.0);
}

TEST(ZsmoothFilterTest, z_output_throws)
{
    PointTable table;
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    BufferReader reader;
    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 0.0);
    reader.addView(view);

    StageFactory f;
    Stage* filter(f.createStage("filters.zsmooth"));
    filter->setInput(reader);
    Options opts;
    opts.add("dim", "Z");
    filter->setOptions(opts);
    EXPECT_THROW(filter->prepare(table), pdal_error);
}
