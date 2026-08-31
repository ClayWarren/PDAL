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

using namespace pdal;

namespace
{

PointViewPtr makeCloud(PointTable& table)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    PointViewPtr view(new PointView(table));
    PointId id = 0;
    for (int i = 0; i < 5; ++i)
        for (int j = 0; j < 5; ++j)
        {
            view->setField(Dimension::Id::X, id, i * 0.5);
            view->setField(Dimension::Id::Y, id, j * 0.5);
            view->setField(Dimension::Id::Z, id, 0.0);
            ++id;
        }
    view->setField(Dimension::Id::X, id, 1000.0);
    view->setField(Dimension::Id::Y, id, 1000.0);
    view->setField(Dimension::Id::Z, id, 1000.0);
    return view;
}

PointViewPtr run(PointTable& table, const Options& opts)
{
    BufferReader reader;
    StageFactory f;
    Stage* filter(f.createStage("filters.outlier"));
    filter->setInput(reader);
    filter->setOptions(opts);
    filter->prepare(table);
    reader.addView(makeCloud(table));
    return *filter->execute(table).begin();
}

} // unnamed namespace

TEST(OutlierFilterTest, noise_class)
{
    PointTable table;
    Options opts;
    opts.add("method", "radius");
    opts.add("radius", 1.0);
    opts.add("class", 18);
    PointViewPtr out = run(table, opts);
    ASSERT_EQ(out->size(), 26u);

    for (PointId i = 0; i < 25; ++i)
        EXPECT_EQ(out->getFieldAs<int>(Dimension::Id::Classification, i), 0);
    EXPECT_EQ(out->getFieldAs<int>(Dimension::Id::Classification, 25), 18);
}
