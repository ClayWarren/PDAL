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
    Stage* filter(f.createStage("filters.radialdensity"));
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

TEST(RadialDensityFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.radialdensity"));
    EXPECT_TRUE(filter);
    EXPECT_EQ(filter->getName(), "filters.radialdensity");
}

TEST(RadialDensityFilterTest, density)
{
    std::vector<std::array<double, 3>> pts = {
        {{0.0, 0.0, 0.0}}, {{0.1, 0.0, 0.0}}, {{0.0, 0.1, 0.0}},
        {{0.0, 0.0, 0.1}}, {{0.1, 0.1, 0.0}}, {{100.0, 100.0, 100.0}}};

    PointTable table;
    Options opts;
    opts.add("radius", 1.0);
    PointViewPtr out = run(table, pts, opts);
    ASSERT_EQ(out->size(), 6u);

    const double factor = 1.0 / ((4.0 / 3.0) * 3.14159 * 1.0);
    for (PointId i = 0; i < 5; ++i)
        EXPECT_NEAR(out->getFieldAs<double>(Dimension::Id::RadialDensity, i),
                    5 * factor, 1e-9);
    EXPECT_NEAR(out->getFieldAs<double>(Dimension::Id::RadialDensity, 5),
                factor, 1e-9);
}
