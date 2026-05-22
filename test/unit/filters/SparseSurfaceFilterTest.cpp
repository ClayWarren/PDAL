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

#include <filters/SparseSurfaceFilter.hpp>
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
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y,
                                  Dimension::Id::Z,
                                  Dimension::Id::Classification});

    BufferReader reader;
    StageFactory f;
    Stage* filter(f.createStage("filters.sparsesurface"));
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

TEST(SparseSurfaceFilterTest, create)
{
    SparseSurfaceFilter filter;
    Options opts;
    opts.add("radius", 1.0);
    filter.setOptions(opts);

    BufferReader reader;
    PointTable table;
    reader.addView(PointViewPtr(new PointView(table)));
    filter.setInput(reader);
    filter.prepare(table);

    EXPECT_EQ(filter.getName(), "filters.sparsesurface");
}

TEST(SparseSurfaceFilterTest, lowest_is_ground_rest_low_noise)
{
    std::vector<std::array<double, 3>> pts = {
        {{0.0, 0.0, 0.0}},  {{0.0, 0.0, 1.0}},  {{0.0, 0.0, 2.0}},
        {{10.0, 0.0, 0.5}}, {{10.0, 0.0, 1.5}}, {{10.0, 0.0, 2.5}}};

    PointTable table;
    Options opts;
    opts.add("radius", 1.0);
    PointViewPtr out = run(table, pts, opts);
    ASSERT_EQ(out->size(), 6u);

    auto cls = [&out](PointId i)
    { return out->getFieldAs<int>(Dimension::Id::Classification, i); };
    EXPECT_EQ(cls(0), 2);
    EXPECT_EQ(cls(1), 7);
    EXPECT_EQ(cls(2), 7);
    EXPECT_EQ(cls(3), 2);
    EXPECT_EQ(cls(4), 7);
    EXPECT_EQ(cls(5), 7);
}

TEST(SparseSurfaceFilterTest, equal_classes_throw)
{
    PointTable table;
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y,
                                  Dimension::Id::Z,
                                  Dimension::Id::Classification});
    BufferReader reader;
    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 0.0);
    reader.addView(view);

    StageFactory f;
    Stage* filter(f.createStage("filters.sparsesurface"));
    filter->setInput(reader);
    Options opts;
    opts.add("ground_class", 5);
    opts.add("low_point_class", 5);
    filter->setOptions(opts);
    EXPECT_THROW(filter->prepare(table), pdal_error);
}
