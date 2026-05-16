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

#include <filters/EstimateRankFilter.hpp>
#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/StageFactory.hpp>

using namespace pdal;

namespace
{

PointViewPtr makeView(PointTable& table)
{
    // Register every dimension -- including the filter's output -- before any
    // point exists, so the layout is final and point storage is not resized
    // (and thus corrupted) when the filter calls addDimensions().
    table.layout()->registerDim(Dimension::Id::X);
    table.layout()->registerDim(Dimension::Id::Y);
    table.layout()->registerDim(Dimension::Id::Z);
    table.layout()->registerDim(Dimension::Id::Rank);
    return PointViewPtr(new PointView(table));
}

} // unnamed namespace

TEST(EstimateRankFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.estimaterank"));
    EXPECT_TRUE(filter);
}

// The rank of a planar neighborhood is 2: two non-negligible eigenvalues.
TEST(EstimateRankFilterTest, planar)
{
    PointTable table;
    PointViewPtr view = makeView(table);

    PointId idx = 0;
    for (int x = 0; x < 5; ++x)
        for (int y = 0; y < 5; ++y, ++idx)
        {
            view->setField(Dimension::Id::X, idx, (double)x);
            view->setField(Dimension::Id::Y, idx, (double)y);
            view->setField(Dimension::Id::Z, idx, 0.0);
        }

    BufferReader r;
    r.addView(view);

    EstimateRankFilter filter;
    filter.setInput(r);
    EXPECT_EQ(filter.getName(), "filters.estimaterank");

    filter.prepare(table);
    PointViewSet viewSet = filter.execute(table);
    PointViewPtr out = *viewSet.begin();

    ASSERT_EQ(out->size(), 25u);
    for (PointId i = 0; i < out->size(); ++i)
        EXPECT_EQ(out->getFieldAs<int>(Dimension::Id::Rank, i), 2);
}

// The rank of a collinear neighborhood is 1: a single non-negligible
// eigenvalue.
TEST(EstimateRankFilterTest, linear)
{
    PointTable table;
    PointViewPtr view = makeView(table);

    for (PointId i = 0; i < 12; ++i)
    {
        view->setField(Dimension::Id::X, i, (double)i);
        view->setField(Dimension::Id::Y, i, 0.0);
        view->setField(Dimension::Id::Z, i, 0.0);
    }

    BufferReader r;
    r.addView(view);

    EstimateRankFilter filter;
    filter.setInput(r);
    filter.prepare(table);
    PointViewSet viewSet = filter.execute(table);
    PointViewPtr out = *viewSet.begin();

    ASSERT_EQ(out->size(), 12u);
    for (PointId i = 0; i < out->size(); ++i)
        EXPECT_EQ(out->getFieldAs<int>(Dimension::Id::Rank, i), 1);
}
