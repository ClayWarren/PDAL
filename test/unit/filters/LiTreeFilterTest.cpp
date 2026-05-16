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

TEST(LiTreeFilterTest, create)
{
    StageFactory f;
    Stage* filter(f.createStage("filters.litree"));
    EXPECT_TRUE(filter);
    EXPECT_EQ(filter->getName(), "filters.litree");
}

TEST(LiTreeFilterTest, missing_hag_throws)
{
    PointTable table;
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    BufferReader reader;
    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 0.0);
    reader.addView(view);

    StageFactory f;
    Stage* filter(f.createStage("filters.litree"));
    filter->setInput(reader);
    EXPECT_THROW(filter->prepare(table), pdal_error);
}

// A single tight, tall cluster (HeightAboveGround rising to 10) is segmented
// into one tree: its points are labeled with ClusterID 1.
TEST(LiTreeFilterTest, segments_a_tree)
{
    PointTable table;
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y,
                                  Dimension::Id::Z,
                                  Dimension::Id::HeightAboveGround});

    BufferReader reader;
    StageFactory f;
    Stage* filter(f.createStage("filters.litree"));
    filter->setInput(reader);
    filter->prepare(table);

    PointViewPtr view(new PointView(table));
    PointId id = 0;
    for (int i = 0; i < 6; ++i)
        for (int j = 0; j < 6; ++j, ++id)
        {
            double hag = 1.0 + (static_cast<double>(id) / 35.0) * 9.0;
            view->setField(Dimension::Id::X, id, i * 0.2);
            view->setField(Dimension::Id::Y, id, j * 0.2);
            view->setField(Dimension::Id::Z, id, hag);
            view->setField(Dimension::Id::HeightAboveGround, id, hag);
        }
    reader.addView(view);

    PointViewPtr out = *filter->execute(table).begin();
    ASSERT_EQ(out->size(), 36u);

    // Characterization: the segmentation assigns exactly 25 of the 36 points
    // to the tree. Any change to the dt threshold, the classify-point test,
    // or the minimum-size gate shifts this count.
    int inTree = 0;
    for (PointId i = 0; i < out->size(); ++i)
        if (out->getFieldAs<int>(Dimension::Id::ClusterID, i) == 1)
            ++inTree;
    EXPECT_EQ(inTree, 25);
}
