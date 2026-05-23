/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
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
 * OF USE, view, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include <pdal/pdal_test_main.hpp>

#include <array>
#include <random>

#include <pdal/PDALUtils.hpp>
#include <pdal/PointView.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include "Support.hpp"

using namespace pdal;

PointViewPtr makeTestView(PointTableRef table, point_count_t cnt = 17)
{
    PointLayoutPtr layout(table.layout());

    layout->registerDim(Dimension::Id::Classification);
    layout->registerDim(Dimension::Id::X);
    layout->registerDim(Dimension::Id::Y);

    PointViewPtr view(new PointView(table));

    // write the data into the view
    for (PointId i = 0; i < cnt; i++)
    {
        const uint8_t x = static_cast<uint8_t>(i + 1);
        const int32_t y = static_cast<int32_t>(i * 10);
        const double z = static_cast<double>(i * 100);

        view->setField(Dimension::Id::Classification, i, x);
        view->setField(Dimension::Id::X, i, y);
        view->setField(Dimension::Id::Y, i, z);
    }
    EXPECT_EQ(view->size(), cnt);
    return view;
}

void verifyTestView(const PointView& view, point_count_t cnt = 17)
{
    // read the view back out
    for (PointId i = 0; i < cnt; i++)
    {
        uint8_t x = view.getFieldAs<uint8_t>(Dimension::Id::Classification, i);
        int32_t y = view.getFieldAs<uint32_t>(Dimension::Id::X, i);
        double z = view.getFieldAs<double>(Dimension::Id::Y, i);

        EXPECT_EQ(x, (uint8_t)(i + 1));
        EXPECT_EQ(y, (int32_t)(i * 10));
        EXPECT_TRUE(
            Utils::compare_approx(z, static_cast<double>(i) * 100.0,
                                  (std::numeric_limits<double>::min)()));
    }
}

constexpr int RustU8 = 0;
constexpr int RustF32 = 8;
constexpr int RustF64 = 9;

pdal_point_view_t* makeRustTestView(point_count_t cnt = 17)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    pdal_point_layout_register_dim(layout, "Classification", RustU8);
    pdal_point_layout_register_dim(layout, "X", RustF64);
    pdal_point_layout_register_dim(layout, "Y", RustF64);

    pdal_point_view_t* view = pdal_point_view_create(layout);
    EXPECT_NE(view, nullptr);

    for (PointId i = 0; i < cnt; i++)
    {
        EXPECT_EQ(pdal_point_view_add_point(view), i);
        pdal_point_view_set_f64(view, i, "Classification", i + 1);
        pdal_point_view_set_f64(view, i, "X", i * 10);
        pdal_point_view_set_f64(view, i, "Y", i * 100);
    }
    EXPECT_EQ(pdal_point_view_length(view), cnt);
    return view;
}

TEST(PointViewTest, getSet)
{
    pdal_point_view_t* view = makeRustTestView(1);
    uint8_t classification = 0;
    int32_t x = 0;
    float y = 0;

    ASSERT_TRUE(
        pdal_point_view_get_u8(view, 0, "Classification", &classification));
    ASSERT_TRUE(pdal_point_view_get_i32(view, 0, "X", &x));
    ASSERT_TRUE(pdal_point_view_get_f32(view, 0, "Y", &y));
    EXPECT_EQ(classification, 1u);
    EXPECT_EQ(x, 0);
    EXPECT_FLOAT_EQ(y, 0.0f);

    pdal_point_view_destroy(view);
}

TEST(PointViewTest, getAsUint8)
{
    pdal_point_view_t* view = makeRustTestView();

    for (int i = 0; i < 3; i++)
    {
        uint8_t x = 0;
        uint8_t y = 0;
        uint8_t z = 0;
        ASSERT_TRUE(pdal_point_view_get_u8(view, i, "Classification", &x));
        ASSERT_TRUE(pdal_point_view_get_u8(view, i, "X", &y));
        ASSERT_TRUE(pdal_point_view_get_u8(view, i, "Y", &z));

        EXPECT_EQ(x, i + 1u);
        EXPECT_EQ(y, i * 10u);
        EXPECT_EQ(z, i * 100u);
    }

    for (int i = 3; i < 17; i++)
    {
        uint8_t x = 0;
        uint8_t y = 0;
        uint8_t z = 0;
        ASSERT_TRUE(pdal_point_view_get_u8(view, i, "Classification", &x));
        ASSERT_TRUE(pdal_point_view_get_u8(view, i, "X", &y));
        EXPECT_FALSE(pdal_point_view_get_u8(view, i, "Y", &z));
        EXPECT_EQ(x, i + 1u);
        EXPECT_EQ(y, i * 10u);
    }

    pdal_point_view_destroy(view);
}

TEST(PointViewTest, getAsInt32)
{
    pdal_point_view_t* view = makeRustTestView();

    for (int i = 0; i < 17; i++)
    {
        int32_t x = 0;
        int32_t y = 0;
        int32_t z = 0;
        ASSERT_TRUE(pdal_point_view_get_i32(view, i, "Classification", &x));
        ASSERT_TRUE(pdal_point_view_get_i32(view, i, "X", &y));
        ASSERT_TRUE(pdal_point_view_get_i32(view, i, "Y", &z));

        EXPECT_EQ(x, i + 1);
        EXPECT_EQ(y, i * 10);
        EXPECT_EQ(z, i * 100);
    }

    pdal_point_view_destroy(view);
}

TEST(PointViewTest, getFloat)
{
    pdal_point_view_t* view = makeRustTestView();

    for (int i = 0; i < 17; i++)
    {
        float x = 0;
        float y = 0;
        float z = 0;
        ASSERT_TRUE(pdal_point_view_get_f32(view, i, "Classification", &x));
        ASSERT_TRUE(pdal_point_view_get_f32(view, i, "X", &y));
        ASSERT_TRUE(pdal_point_view_get_f32(view, i, "Y", &z));

        EXPECT_FLOAT_EQ(x, i + 1.0f);
        EXPECT_FLOAT_EQ(y, i * 10.0f);
        EXPECT_FLOAT_EQ(z, i * 100.0f);
    }

    pdal_point_view_destroy(view);
}

TEST(PointViewTest, calculateBounds)
{
    PointTable table;
    PointLayoutPtr layout(table.layout());
    layout->registerDim(Dimension::Id::X);
    layout->registerDim(Dimension::Id::Y);
    layout->registerDim(Dimension::Id::Z);

    PointView view(table);
    view.setField(Dimension::Id::X, 0, -10.0);
    view.setField(Dimension::Id::Y, 0, 5.0);
    view.setField(Dimension::Id::Z, 0, 100.0);
    view.setField(Dimension::Id::X, 1, 20.0);
    view.setField(Dimension::Id::Y, 1, -15.0);
    view.setField(Dimension::Id::Z, 1, -50.0);
    view.setField(Dimension::Id::X, 2, 3.0);
    view.setField(Dimension::Id::Y, 2, 7.0);
    view.setField(Dimension::Id::Z, 2, 25.0);

    BOX2D box2d;
    view.calculateBounds(box2d);
    EXPECT_DOUBLE_EQ(box2d.minx, -10.0);
    EXPECT_DOUBLE_EQ(box2d.maxx, 20.0);
    EXPECT_DOUBLE_EQ(box2d.miny, -15.0);
    EXPECT_DOUBLE_EQ(box2d.maxy, 7.0);

    BOX3D box3d;
    view.calculateBounds(box3d);
    EXPECT_DOUBLE_EQ(box3d.minx, -10.0);
    EXPECT_DOUBLE_EQ(box3d.maxx, 20.0);
    EXPECT_DOUBLE_EQ(box3d.miny, -15.0);
    EXPECT_DOUBLE_EQ(box3d.maxy, 7.0);
    EXPECT_DOUBLE_EQ(box3d.minz, -50.0);
    EXPECT_DOUBLE_EQ(box3d.maxz, 100.0);
}

TEST(PointViewTest, pointRef)
{
    PointTable table;
    PointLayoutPtr layout(table.layout());
    layout->registerDim(Dimension::Id::X);
    layout->registerDim(Dimension::Id::Y);

    PointView view(table);
    view.setField(Dimension::Id::X, 0, 10.0);
    view.setField(Dimension::Id::Y, 0, 20.0);
    view.setField(Dimension::Id::X, 1, 30.0);
    view.setField(Dimension::Id::Y, 1, 40.0);

    PointRef point(view, 0);
    EXPECT_DOUBLE_EQ(point.getFieldAs<double>(Dimension::Id::X), 10.0);
    point.setField(Dimension::Id::X, 15.0);
    EXPECT_DOUBLE_EQ(view.getFieldAs<double>(Dimension::Id::X, 0), 15.0);

    point.setPointId(1);
    EXPECT_EQ(point.pointId(), 1u);
    EXPECT_DOUBLE_EQ(point.getFieldAs<double>(Dimension::Id::Y), 40.0);

    PointRef added(view, view.size());
    added.setField(Dimension::Id::X, 50.0);
    added.setField(Dimension::Id::Y, 60.0);
    EXPECT_EQ(view.size(), 3u);
    EXPECT_DOUBLE_EQ(view.getFieldAs<double>(Dimension::Id::X, 2), 50.0);
    EXPECT_DOUBLE_EQ(view.getFieldAs<double>(Dimension::Id::Y, 2), 60.0);

    PointRef first(view, 0);
    PointRef second(view, 1);
    PointRef::swap(first, second);
    EXPECT_EQ(first.pointId(), 1u);
    EXPECT_EQ(second.pointId(), 0u);
    EXPECT_DOUBLE_EQ(first.getFieldAs<double>(Dimension::Id::X), 30.0);
    EXPECT_DOUBLE_EQ(second.getFieldAs<double>(Dimension::Id::X), 15.0);
    EXPECT_DOUBLE_EQ(view.getFieldAs<double>(Dimension::Id::X, 0), 30.0);
    EXPECT_DOUBLE_EQ(view.getFieldAs<double>(Dimension::Id::X, 1), 15.0);

    const PointRef constFirst(view, 0);
    const PointRef constAdded(view, 2);
    PointRef::swap(constFirst, constAdded);
    EXPECT_DOUBLE_EQ(view.getFieldAs<double>(Dimension::Id::X, 0), 50.0);
    EXPECT_DOUBLE_EQ(view.getFieldAs<double>(Dimension::Id::X, 2), 30.0);

    PointRef tablePoint(table);
    tablePoint.setPointId(7);
    EXPECT_EQ(tablePoint.pointId(), 7u);
}

TEST(PointViewTest, bigfile)
{
    PointTable table;

    point_count_t NUM_PTS = 1000000;

    PointLayoutPtr layout(table.layout());

    layout->registerDim(Dimension::Id::X);
    layout->registerDim(Dimension::Id::Y);
    layout->registerDim(Dimension::Id::Z);

    PointView view(table);

    for (PointId id = 0; id < NUM_PTS; ++id)
    {
        view.setField(Dimension::Id::X, id, id);
        view.setField(Dimension::Id::Y, id, 2 * id);
        view.setField(Dimension::Id::Z, id, -(int)id);
    }

    for (PointId id = 0; id < NUM_PTS; ++id)
    {
        EXPECT_EQ(view.getFieldAs<PointId>(Dimension::Id::X, id), id);
        EXPECT_EQ(view.getFieldAs<PointId>(Dimension::Id::Y, id), id * 2);
        EXPECT_EQ(view.getFieldAs<int>(Dimension::Id::Z, id), -(int)id);
    }

    // Test some random access.
    std::unique_ptr<PointId[]> ids(new PointId[NUM_PTS]);
    for (PointId idx = 0; idx < NUM_PTS; ++idx)
        ids[idx] = idx;
    // Do a bunch of random swaps.
    std::default_random_engine generator;
    std::uniform_int_distribution<PointId> distribution(0, NUM_PTS - 1);
    for (PointId idx = 0; idx < NUM_PTS; ++idx)
    {
        PointId y = distribution(generator);
        PointId temp = ids[idx];
        ids[idx] = ids[y];
        ids[y] = temp;
    }

    for (PointId idx = 0; idx < NUM_PTS; ++idx)
    {
        PointId id = ids[idx];
        view.setField(Dimension::Id::X, id, idx);
        view.setField(Dimension::Id::Y, id, 2 * idx);
        view.setField(Dimension::Id::Z, id, -(int)idx);
    }

    for (PointId idx = 0; idx < NUM_PTS; ++idx)
    {
        PointId id = ids[idx];
        EXPECT_EQ(view.getFieldAs<PointId>(Dimension::Id::X, id), idx);
        EXPECT_EQ(view.getFieldAs<PointId>(Dimension::Id::Y, id), idx * 2);
        EXPECT_EQ(view.getFieldAs<int>(Dimension::Id::Z, id), -(int)idx);
    }
}

TEST(PointViewTest, order)
{
    PointTable table;

    const size_t COUNT(1000);
    std::array<PointViewPtr, COUNT> views;

    std::random_device dev;
    std::mt19937 generator(dev());

    for (size_t i = 0; i < COUNT; ++i)
        views[i] = PointViewPtr(new PointView(table));
    std::shuffle(views.begin(), views.end(), generator);

    PointViewSet set;
    for (size_t i = 0; i < COUNT; ++i)
        set.insert(views[i]);

    PointViewSet::iterator pi;
    for (auto si = set.begin(); si != set.end(); ++si)
    {
        if (si != set.begin())
            EXPECT_TRUE((*pi)->id() < (*si)->id());
        pi = si;
    }
}

TEST(PointViewTest, issue1264)
{
    PointTable t;
    PointLayoutPtr layout(t.layout());
    Dimension::Id foo = layout->assignDim("foo", Dimension::Type::Unsigned8);
    Dimension::Id bar = layout->assignDim("bar", Dimension::Type::Signed8);
    layout->finalize();

    PointView v(t);
    double d(250.0);
    v.setField(foo, 0, d);
    d = v.getFieldAs<double>(foo, 0);
    EXPECT_DOUBLE_EQ(d, 250.0);
    d = 123.0;
    v.setField(bar, 0, d);
    d = v.getFieldAs<double>(bar, 0);
    EXPECT_DOUBLE_EQ(d, 123.0);
    d = -120.23456;
    v.setField(bar, 0, d);
    d = v.getFieldAs<double>(bar, 0);
    EXPECT_DOUBLE_EQ(d, -120.0);
    d = 260.0;
    EXPECT_THROW(v.setField(foo, 0, d), pdal_error);
}

TEST(PointViewTest, getFloatNan)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    pdal_point_layout_register_dim(layout, "ScanAngleRank", RustF32);
    pdal_point_view_t* view = pdal_point_view_create(layout);
    ASSERT_NE(view, nullptr);

    const float scanAngleRank = std::numeric_limits<float>::quiet_NaN();
    EXPECT_EQ(pdal_point_view_add_point(view), 0u);
    pdal_point_view_set_f64(view, 0, "ScanAngleRank", scanAngleRank);

    float value = 0.0f;
    ASSERT_TRUE(pdal_point_view_get_f32(view, 0, "ScanAngleRank", &value));
    EXPECT_TRUE(std::isnan(value));

    pdal_point_view_destroy(view);
}

// Per discussions with @abellgithub
// (https://github.com/gadomski/PDAL/commit/c1d54e56e2de841d37f2a1b1c218ed723053f6a9#commitcomment-14415138)
// we only do bounds checking on `PointView`s when in debug mode.
#ifndef NDEBUG
TEST(PointViewDeathTest, out_of_bounds)
{
    PointTable point_table;
    auto point_view = makeTestView(point_table, 1);
    EXPECT_DEATH(point_view->getFieldAs<uint8_t>(Dimension::Id::X, 1),
                 "< m_size");
}
#endif
