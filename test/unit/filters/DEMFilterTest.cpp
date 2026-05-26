/******************************************************************************
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and this disclaimer.
 *     * Redistributions in binary form must reproduce the above
 *       copyright notice, this list of conditions and this disclaimer in
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
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include <pdal/pdal_test_main.hpp>

#include <io/BufferReader.hpp>
#include <pdal/PointTable.hpp>
#include <pdal/StageFactory.hpp>

#include "Support.hpp"

namespace pdal
{

TEST(DEMFilterTest, KeepsPointsInsideRasterRelativeLimits)
{
    PointTable table;
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 440750.0);
    view->setField(Dimension::Id::Y, 0, 3751290.0);
    view->setField(Dimension::Id::Z, 0, 200.0);
    view->setField(Dimension::Id::X, 1, 440750.0);
    view->setField(Dimension::Id::Y, 1, 3751290.0);
    view->setField(Dimension::Id::Z, 1, 208.0);

    BufferReader reader;
    reader.addView(view);

    StageFactory factory;
    Stage& filter = *factory.createStage("filters.dem");
    Options opts;
    opts.add("raster", Support::datapath("gdal/float32.tif"));
    opts.add("limits", "Z[0:100]");
    filter.setInput(reader);
    filter.setOptions(opts);

    filter.prepare(table);
    PointViewSet views = filter.execute(table);
    ASSERT_EQ(views.size(), 1u);
    PointViewPtr result = *views.begin();
    ASSERT_EQ(result->size(), 1u);
    EXPECT_DOUBLE_EQ(result->getFieldAs<double>(Dimension::Id::Z, 0), 200.0);
}

} // namespace pdal
