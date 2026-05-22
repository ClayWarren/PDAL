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

PointViewPtr makePlane(PointTable& table)
{
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    PointViewPtr view(new PointView(table));
    PointId id = 0;
    for (double x : {0.0, 1.0, 2.0})
        for (double y : {0.0, 1.0, 2.0})
        {
            view->setField(Dimension::Id::X, id, x);
            view->setField(Dimension::Id::Y, id, y);
            view->setField(Dimension::Id::Z, id, 0.0);
            ++id;
        }
    return view;
}

PointViewPtr run(PointTable& table, const Options& opts)
{
    BufferReader reader;
    StageFactory f;
    Stage* filter(f.createStage("filters.eigenvalues"));
    filter->setInput(reader);
    filter->setOptions(opts);
    filter->prepare(table);
    reader.addView(makePlane(table));
    return *filter->execute(table).begin();
}

} // unnamed namespace

TEST(EigenvaluesFilterTest, create)
{
    EigenvaluesFilter filter;
    BufferReader reader;
    PointTable table;
    table.layout()->registerDims(
        {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z});
    reader.addView(PointViewPtr(new PointView(table)));
    filter.setInput(reader);
    filter.prepare(table);

    EXPECT_EQ(filter.getName(), "filters.eigenvalues");
}

TEST(EigenvaluesFilterTest, planar_neighborhood)
{
    PointTable table;
    Options opts;
    opts.add("knn", 8);
    PointViewPtr out = run(table, opts);

    for (PointId i = 0; i < out->size(); ++i)
    {
        EXPECT_NEAR(out->getFieldAs<double>(Dimension::Id::Eigenvalue0, i), 0.0,
                    1e-12);
        EXPECT_GT(out->getFieldAs<double>(Dimension::Id::Eigenvalue1, i), 0.0);
        EXPECT_GT(out->getFieldAs<double>(Dimension::Id::Eigenvalue2, i), 0.0);
    }
}

TEST(EigenvaluesFilterTest, normalized_eigenvalues_sum_to_one)
{
    PointTable table;
    Options opts;
    opts.add("knn", 8);
    opts.add("normalize", true);
    PointViewPtr out = run(table, opts);

    for (PointId i = 0; i < out->size(); ++i)
    {
        double sum = out->getFieldAs<double>(Dimension::Id::Eigenvalue0, i) +
                     out->getFieldAs<double>(Dimension::Id::Eigenvalue1, i) +
                     out->getFieldAs<double>(Dimension::Id::Eigenvalue2, i);
        EXPECT_NEAR(sum, 1.0, 1e-12);
    }
}
