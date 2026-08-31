/******************************************************************************
* Copyright (c) 2019, Hobu Inc. (info@hobu.co)
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
*     * Neither the name of Hobu, Inc. nor the
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

#include <vector>

// NOTE: The test data has an accompanying jpg that depicts the points,
//  their triangulation and the interesting barycentric calculation.

namespace pdal
{

namespace
{

struct HagPoint
{
    double z;
    uint8_t classification;
    double hag;
};

std::vector<HagPoint> hagPoints(const PointViewSet& views)
{
    std::vector<HagPoint> output;
    for (PointViewPtr view : views)
        for (PointId i = 0; i < view->size(); ++i)
            output.push_back(
                {view->getFieldAs<double>(Dimension::Id::Z, i),
                 view->getFieldAs<uint8_t>(Dimension::Id::Classification, i),
                 view->getFieldAs<double>(Dimension::Id::HeightAboveGround,
                                          i)});
    return output;
}

} // unnamed namespace

TEST(HAGFilterTest, delaunay)
{
    Options ro;
    ro.add("filename", Support::datapath("filters/hagtest.txt"));

    StageFactory factory;
    Stage& r = *(factory.createStage("readers.text"));
    r.setOptions(ro);

    Options fo;
    fo.add("count", 10);
    Stage& f = *(factory.createStage("filters.hag_delaunay"));
    f.setInput(r);
    f.setOptions(fo);

    PointTable t1;
    f.prepare(t1);
    PointViewSet s = f.execute(t1);
    PointViewPtr v = *s.begin();

    for (PointId i = 0; i < v->size(); ++i)
    {
        double x = v->getFieldAs<double>(Dimension::Id::X, i);
        double y = v->getFieldAs<double>(Dimension::Id::Y, i);
        double z = v->getFieldAs<double>(Dimension::Id::Z, i);
        double hag = v->getFieldAs<double>(Dimension::Id::HeightAboveGround, i);
        uint8_t c = v->getFieldAs<uint8_t>(Dimension::Id::Classification, i);
        if (c == ClassLabel::Ground)
            EXPECT_EQ(hag, 0);
        auto check = [&x, &y, &z, &hag](double xv, double yv, double zv,
                                        double hagv)
        {
            EXPECT_EQ(x, xv) << "Bad X Value";
            EXPECT_EQ(y, yv) << "Bad Y Value";
            EXPECT_EQ(z, zv) << "Bad Z Value";
            EXPECT_EQ(hag, hagv) << "Bad HAG Value";
        };

        if (i == 0)
            check (-2, 4, 20, 10);
        if (i == 1)
            check(4, 1, 20, 11);
        if (i == 2)
            check(2, 3, 20, 14);
        if (i == 3)
            check(4, 4, 20, 16);
    }
}

TEST(HAGFilterTest, neighbors)
{
    Options ro;
    ro.add("filename", Support::datapath("filters/hagtest.txt"));

    StageFactory factory;
    Stage& r = *(factory.createStage("readers.text"));
    r.setOptions(ro);

    Options fo;
    fo.add("count", 2);
    Stage& f = *(factory.createStage("filters.hag_nn"));
    f.setInput(r);
    f.setOptions(fo);

    PointTable t1;
    f.prepare(t1);
    PointViewSet s = f.execute(t1);
    PointViewPtr v = *s.begin();

    for (PointId i = 0; i < v->size(); ++i)
    {
        double x = v->getFieldAs<double>(Dimension::Id::X, i);
        double y = v->getFieldAs<double>(Dimension::Id::Y, i);
        double z = v->getFieldAs<double>(Dimension::Id::Z, i);
        double hag = v->getFieldAs<double>(Dimension::Id::HeightAboveGround, i);
        uint8_t c = v->getFieldAs<uint8_t>(Dimension::Id::Classification, i);
        if (c == ClassLabel::Ground)
            EXPECT_EQ(hag, 0);
        auto check = [&x, &y, &z, &hag](double xv, double yv, double zv,
                                        double hagv)
        {
            EXPECT_EQ(x, xv) << "Bad X Value";
            EXPECT_EQ(y, yv) << "Bad Y Value";
            EXPECT_EQ(z, zv) << "Bad Z Value";
            EXPECT_DOUBLE_EQ(hag, hagv) << "Bad HAG Value";
        };

        if (i == 0)
            check (-2, 4, 20, 10);
        if (i == 1)
            check(4, 1, 20, 10);
        if (i == 2)
            check(2, 3, 20, 14.8);
        if (i == 3)
            check(4, 4, 20, 15);
    }
}

TEST(HAGFilterTest, closest)
{
    Options ro;
    ro.add("filename", Support::datapath("filters/hagtest.txt"));

    StageFactory factory;
    Stage& r = *(factory.createStage("readers.text"));
    r.setOptions(ro);

    Options fo;
    fo.add("count", 1);
    Stage& f = *(factory.createStage("filters.hag_nn"));
    f.setInput(r);
    f.setOptions(fo);

    PointTable t1;
    f.prepare(t1);
    PointViewSet s = f.execute(t1);
    PointViewPtr v = *s.begin();

    for (PointId i = 0; i < v->size(); ++i)
    {
        double x = v->getFieldAs<double>(Dimension::Id::X, i);
        double y = v->getFieldAs<double>(Dimension::Id::Y, i);
        double z = v->getFieldAs<double>(Dimension::Id::Z, i);
        double hag = v->getFieldAs<double>(Dimension::Id::HeightAboveGround, i);
        uint8_t c = v->getFieldAs<uint8_t>(Dimension::Id::Classification, i);
        if (c == ClassLabel::Ground)
            EXPECT_EQ(hag, 0);
        auto check = [&x, &y, &z, &hag](double xv, double yv, double zv,
                                        double hagv)
        {
            EXPECT_EQ(x, xv) << "Bad X Value";
            EXPECT_EQ(y, yv) << "Bad Y Value";
            EXPECT_EQ(z, zv) << "Bad Z Value";
            EXPECT_DOUBLE_EQ(hag, hagv) << "Bad HAG Value";
        };

        if (i == 0)
            check (-2, 4, 20, 10);
        if (i == 1)
            check(4, 1, 20, 10);
        if (i == 2)
            check(2, 3, 20, 16);
        if (i == 3)
            check(4, 4, 20, 16);
    }
}

TEST(HAGFilterTest, dem)
{
    PointTable table;
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y,
                                  Dimension::Id::Z,
                                  Dimension::Id::Classification});

    BufferReader reader;
    StageFactory factory;
    Stage& f = *(factory.createStage("filters.hag_dem"));
    Options opts;
    opts.add("raster", Support::datapath("gdal/float32.tif"));
    f.setInput(reader);
    f.setOptions(opts);
    f.prepare(table);

    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 440750.0);
    view->setField(Dimension::Id::Y, 0, 3751290.0);
    view->setField(Dimension::Id::Z, 0, 200.0);
    view->setField(Dimension::Id::Classification, 0, ClassLabel::Ground);
    view->setField(Dimension::Id::X, 1, 440750.0);
    view->setField(Dimension::Id::Y, 1, 3751290.0);
    view->setField(Dimension::Id::Z, 1, 200.0);
    view->setField(Dimension::Id::Classification, 1, ClassLabel::Unclassified);

    reader.addView(view);
    PointViewSet s = f.execute(table);
    std::vector<HagPoint> points = hagPoints(s);

    ASSERT_EQ(points.size(), 2u);
    bool sawGround = false;
    bool sawUnclassified = false;
    for (const HagPoint& point : points)
    {
        EXPECT_DOUBLE_EQ(point.z, 200.0);
        if (point.classification == ClassLabel::Ground)
        {
            sawGround = true;
            EXPECT_DOUBLE_EQ(point.hag, 0.0);
        }
        else
        {
            EXPECT_EQ(point.classification, ClassLabel::Unclassified);
            sawUnclassified = true;
            EXPECT_DOUBLE_EQ(point.hag, 93.0);
        }
    }
    EXPECT_TRUE(sawGround);
    EXPECT_TRUE(sawUnclassified);
}

TEST(HAGFilterTest, dem_clamps)
{
    PointTable table;
    table.layout()->registerDims({Dimension::Id::X, Dimension::Id::Y,
                                  Dimension::Id::Z,
                                  Dimension::Id::Classification});

    BufferReader reader;
    StageFactory factory;
    Stage& f = *(factory.createStage("filters.hag_dem"));
    Options opts;
    opts.add("raster", Support::datapath("gdal/float32.tif"));
    opts.add("min_clamp", -5.0);
    opts.add("max_clamp", 10.0);
    f.setInput(reader);
    f.setOptions(opts);
    f.prepare(table);

    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 440750.0);
    view->setField(Dimension::Id::Y, 0, 3751290.0);
    view->setField(Dimension::Id::Z, 0, 140.0);
    view->setField(Dimension::Id::Classification, 0, ClassLabel::Unclassified);
    view->setField(Dimension::Id::X, 1, 440750.0);
    view->setField(Dimension::Id::Y, 1, 3751290.0);
    view->setField(Dimension::Id::Z, 1, 100.0);
    view->setField(Dimension::Id::Classification, 1, ClassLabel::Unclassified);

    reader.addView(view);
    PointViewSet s = f.execute(table);
    std::vector<HagPoint> points = hagPoints(s);

    ASSERT_EQ(points.size(), 2u);
    bool sawMaxClamp = false;
    bool sawMinClamp = false;
    for (const HagPoint& point : points)
    {
        EXPECT_EQ(point.classification, ClassLabel::Unclassified);
        if (point.z == 140.0)
        {
            sawMaxClamp = true;
            EXPECT_DOUBLE_EQ(point.hag, 10.0);
        }
        else
        {
            EXPECT_DOUBLE_EQ(point.z, 100.0);
            sawMinClamp = true;
            EXPECT_DOUBLE_EQ(point.hag, -5.0);
        }
    }
    EXPECT_TRUE(sawMaxClamp);
    EXPECT_TRUE(sawMinClamp);
}

// Should add tests for exact match in neighbors case and for
// max_distance in neighbors case.

} // namespace pdal
