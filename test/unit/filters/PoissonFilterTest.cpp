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
 *     * Redistributions in binary form must reproduce the above
 *       copyright notice, this list of conditions and the following
 *       disclaimer in the documentation and/or other materials provided
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

#include <filters/PoissonFilter.hpp>
#include <io/BufferReader.hpp>

namespace pdal
{

TEST(PoissonFilterTest, partialNormalsThrow)
{
    PointTable table;
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y,
                                  Dimension::Id::Z, Dimension::Id::NormalX});

    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 0.0);
    view->setField(Dimension::Id::Y, 0, 0.0);
    view->setField(Dimension::Id::Z, 0, 0.0);
    view->setField(Dimension::Id::NormalX, 0, 1.0);

    BufferReader reader;
    reader.addView(view);

    PoissonFilter filter;
    filter.setInput(reader);

    EXPECT_THROW(filter.prepare(table), pdal_error);
}

TEST(PoissonFilterTest, registersMissingNormalDimensions)
{
    PointTable table;
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    PointViewPtr view(new PointView(table));
    BufferReader reader;
    reader.addView(view);

    PoissonFilter filter;
    filter.setInput(reader);
    filter.prepare(table);

    EXPECT_TRUE(table.layout()->hasDim(Dimension::Id::NormalX));
    EXPECT_TRUE(table.layout()->hasDim(Dimension::Id::NormalY));
    EXPECT_TRUE(table.layout()->hasDim(Dimension::Id::NormalZ));
}

} // namespace pdal
