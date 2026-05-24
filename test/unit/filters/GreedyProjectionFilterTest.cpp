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

#include <filters/GreedyProjection.hpp>
#include <io/BufferReader.hpp>
#include <pdal/PointView.hpp>

namespace pdal
{

namespace
{

PointViewPtr planeView(PointTableRef table)
{
    PointLayoutPtr layout = table.layout();
    layout->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});

    PointViewPtr view(new PointView(table));
    for (double y = 0.0; y < 3.0; y += 1.0)
    {
        for (double x = 0.0; x < 3.0; x += 1.0)
        {
            PointId id = view->size();
            view->setField(Dimension::Id::X, id, x);
            view->setField(Dimension::Id::Y, id, y);
            view->setField(Dimension::Id::Z, id, 0.0);
        }
    }
    return view;
}

} // namespace

TEST(GreedyProjectionFilterTest, invalidOptionsThrow)
{
    PointTable table;
    BufferReader reader;
    reader.addView(planeView(table));

    GreedyProjection filter;
    Options options;
    options.add("multiplier", 2.0);
    options.add("radius", 0.0);
    filter.setOptions(options);
    filter.setInput(reader);

    EXPECT_THROW(filter.prepare(table), pdal_error);
}

TEST(GreedyProjectionFilterTest, planarPointsProduceMesh)
{
    PointTable table;
    BufferReader reader;
    reader.addView(planeView(table));

    GreedyProjection filter;
    Options options;
    options.add("multiplier", 2.5);
    options.add("radius", 2.0);
    options.add("num_neighbors", 8);
    filter.setOptions(options);
    filter.setInput(reader);

    filter.prepare(table);
    PointViewSet views = filter.execute(table);
    ASSERT_EQ(views.size(), 1u);
    PointViewPtr out = *views.begin();

    TriangularMesh* mesh = out->mesh("filters.greedyprojection");
    ASSERT_NE(mesh, nullptr);
    ASSERT_GT(mesh->size(), 0u);
    for (size_t i = 0; i < mesh->size(); ++i)
    {
        const Triangle& tri = (*mesh)[i];
        EXPECT_LT(tri.m_a, out->size());
        EXPECT_LT(tri.m_b, out->size());
        EXPECT_LT(tri.m_c, out->size());
    }
}

} // namespace pdal
