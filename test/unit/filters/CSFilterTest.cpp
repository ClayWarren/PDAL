/******************************************************************************
 *
 * All rights reserved.
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
#include <pdal/StageFactory.hpp>

#include "Support.hpp"

using namespace pdal;

TEST(CSFilterTest, stageCreation)
{
    StageFactory factory;
    EXPECT_NO_THROW(factory.createStage("filters.csf"));
}

TEST(CSFilterTest, emptyView)
{
    PointTable table;
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    PointViewPtr view(new PointView(table));
    BufferReader reader;
    reader.addView(view);

    StageFactory factory;
    Stage* filter(factory.createStage("filters.csf"));
    filter->setInput(reader);
    filter->prepare(table);

    PointViewSet s = filter->execute(table);
    EXPECT_EQ(s.size(), 0u);
}

TEST(CSFilterTest, equalClassesThrowWhenOnlyGroundIsFalse)
{
    PointTable table;
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y,
                                  Dimension::Id::Z,
                                  Dimension::Id::Classification});

    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 0.0);
    view->setField(Dimension::Id::Y, 0, 0.0);
    view->setField(Dimension::Id::Z, 0, 0.0);
    view->setField(Dimension::Id::Classification, 0, ClassLabel::Unclassified);

    BufferReader reader;
    reader.addView(view);

    StageFactory factory;
    Stage* filter(factory.createStage("filters.csf"));
    Options options;
    options.add("ground_class", 2);
    options.add("other_class", 2);
    options.add("only_ground", false);
    filter->setOptions(options);
    filter->setInput(reader);

    EXPECT_THROW(filter->prepare(table), pdal_error);
}

TEST(CSFilterTest, invalidIgnoredDimensionThrows)
{
    PointTable table;
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y,
                                  Dimension::Id::Z,
                                  Dimension::Id::Classification});

    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 0.0);
    view->setField(Dimension::Id::Y, 0, 0.0);
    view->setField(Dimension::Id::Z, 0, 0.0);
    view->setField(Dimension::Id::Classification, 0, ClassLabel::Unclassified);

    BufferReader reader;
    reader.addView(view);

    StageFactory factory;
    Stage* filter(factory.createStage("filters.csf"));
    Options options;
    options.add("ignore", "NoSuchDim[1:2]");
    filter->setOptions(options);
    filter->setInput(reader);

    EXPECT_THROW(filter->prepare(table), pdal_error);
}
