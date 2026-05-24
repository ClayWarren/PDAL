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
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include <pdal/pdal_test_main.hpp>

#include "Support.hpp"
#include <io/LasReader.hpp>
#include <pdal/PointTable.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

TEST(PointTable, resolveType)
{
    using namespace Dimension;

    PointTable table;
    PointLayoutPtr layout(table.layout());

    // Start with a default-defined dimension.
    layout->registerDim(Id::X);
    EXPECT_EQ(layout->dimSize(Id::X), 8u);
    EXPECT_EQ(layout->dimType(Id::X), Type::Double);

    layout->registerDim(Id::X, Type::Signed32);
    EXPECT_EQ(layout->dimSize(Id::X), 8u);
    EXPECT_EQ(layout->dimType(Id::X), Type::Double);

    layout->registerDim(Dimension::Id::X, Type::Unsigned8);
    EXPECT_EQ(layout->dimSize(Id::X), 8u);
    EXPECT_EQ(layout->dimType(Id::X), Type::Double);

    /// Build as we go.
    layout->registerDim(Id::Intensity, Type::Unsigned8);
    EXPECT_EQ(layout->dimSize(Id::Intensity), 1u);
    EXPECT_EQ(layout->dimType(Id::Intensity), Type::Unsigned8);

    layout->registerDim(Id::Intensity, Type::Unsigned8);
    EXPECT_EQ(layout->dimSize(Id::Intensity), 1u);
    EXPECT_EQ(layout->dimType(Id::Intensity), Type::Unsigned8);

    layout->registerDim(Id::Intensity, Type::Signed8);
    // Signed 8 and Unsigned 8 should yield signed 16.
    EXPECT_EQ(layout->dimSize(Id::Intensity), 2u);
    EXPECT_EQ(layout->dimType(Id::Intensity), Type::Signed16);

    layout->registerDim(Id::Intensity, Type::Signed16);
    EXPECT_EQ(layout->dimSize(Id::Intensity), 2u);
    EXPECT_EQ(layout->dimType(Id::Intensity), Type::Signed16);

    layout->registerDim(Id::Intensity, Type::Float);
    EXPECT_EQ(layout->dimSize(Id::Intensity), 4u);
    EXPECT_EQ(layout->dimType(Id::Intensity), Type::Float);

    layout->registerDim(Id::Intensity, Type::Double);
    EXPECT_EQ(layout->dimSize(Id::Intensity), 8u);
    EXPECT_EQ(layout->dimType(Id::Intensity), Type::Double);

    ///
    layout->registerDim(Id::Red, Type::Unsigned16);
    EXPECT_EQ(layout->dimSize(Id::Red), 2u);
    EXPECT_EQ(layout->dimType(Id::Red), Type::Unsigned16);

    layout->registerDim(Id::Red, Type::Signed8);
    EXPECT_EQ(layout->dimSize(Id::Red), 4u);
    EXPECT_EQ(layout->dimType(Id::Red), Type::Signed32);

    layout->registerDim(Id::Red, Type::Signed16);
    EXPECT_EQ(layout->dimSize(Id::Red), 4u);
    EXPECT_EQ(layout->dimType(Id::Red), Type::Signed32);

    layout->registerDim(Id::Red, Type::Double);
    EXPECT_EQ(layout->dimSize(Id::Red), 8u);
    EXPECT_EQ(layout->dimType(Id::Red), Type::Double);
}

TEST(PointTable, userView)
{
    class UserTable : public PointTable
    {
    private:
        double m_x;
        double m_y;
        double m_z;

    public:
        PointId addPoint() override
        {
            return 0;
        }
        char* getPoint(PointId idx) override
        {
            return nullptr;
        }
        void setFieldInternal(Dimension::Id id, PointId idx,
                              const void* value) override
        {
            if (id == Dimension::Id::X)
                m_x = *(const double*)value;
            else if (id == Dimension::Id::Y)
                m_y = *(const double*)value;
            else if (id == Dimension::Id::Z)
                m_z = *(const double*)value;
        }
        void getFieldInternal(Dimension::Id id, PointId idx,
                              void* value) const override
        {
            if (id == Dimension::Id::X)
                *(double*)value = m_x;
            else if (id == Dimension::Id::Y)
                *(double*)value = m_y;
            else if (id == Dimension::Id::Z)
                *(double*)value = m_z;
        }
    };

    LasReader reader;

    Options opts;
    opts.add("filename", Support::datapath("las/simple.las"));
    opts.add("count", 100);

    reader.setOptions(opts);

    PointTable defTable;
    reader.prepare(defTable);
    PointViewSet viewSet = reader.execute(defTable);
    PointViewPtr defView = *viewSet.begin();

    bool called(false);
    auto readCb = [defView, &called](PointView& customView, PointId id)
    {
        called = true;
        double xDef = defView->getFieldAs<double>(Dimension::Id::X, id);
        double yDef = defView->getFieldAs<double>(Dimension::Id::Y, id);
        double zDef = defView->getFieldAs<double>(Dimension::Id::Z, id);

        double x = customView.getFieldAs<double>(Dimension::Id::X, id);
        double y = customView.getFieldAs<double>(Dimension::Id::Y, id);
        double z = customView.getFieldAs<double>(Dimension::Id::Z, id);

        EXPECT_DOUBLE_EQ(xDef, x);
        EXPECT_DOUBLE_EQ(yDef, y);
        EXPECT_DOUBLE_EQ(zDef, z);
    };

    reader.setReadCb(readCb);
    UserTable table;

    reader.prepare(table);
    reader.execute(table);
    EXPECT_TRUE(called);
}

TEST(PointTable, srs)
{
    const char* srsText1 =
        "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS "
        "84\",6378137,298.257223563,AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY["
        "\"EPSG\",\"6326\"]],PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\","
        "\"8901\"]],UNIT[\"degree\",0.0174532925199433,AUTHORITY[\"EPSG\","
        "\"9122\"]],AUTHORITY[\"EPSG\",\"4326\"]]";

    const char* srsText2 =
        "PROJCS[\"WGS 84 / UTM zone 17N\",GEOGCS[\"WGS "
        "84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS "
        "84\",6378137,298.257223563,AUTHORITY[\"EPSG\",\"7030\"]],AUTHORITY["
        "\"EPSG\",\"6326\"]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0."
        "0174532925199433],AUTHORITY[\"EPSG\",\"4326\"]],PROJECTION["
        "\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],"
        "PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996]"
        ",PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],"
        "UNIT[\"metre\",1,AUTHORITY[\"EPSG\",\"9001\"]],AUTHORITY[\"EPSG\","
        "\"32617\"]]";

    pdal_spatial_reference_t* srs1 = pdal_spatial_reference_create(srsText1);
    pdal_spatial_reference_t* srs2 = pdal_spatial_reference_create(srsText2);
    pdal_spatial_reference_list_t* list = pdal_spatial_reference_list_create();
    ASSERT_NE(srs1, nullptr);
    ASSERT_NE(srs2, nullptr);
    ASSERT_NE(list, nullptr);

    pdal_spatial_reference_list_add(list, srs1);
    pdal_spatial_reference_list_add(list, srs1);
    EXPECT_TRUE(pdal_spatial_reference_list_unique(list));
    EXPECT_EQ(pdal_spatial_reference_list_size(list), 1u);
    pdal_spatial_reference_t* any = pdal_spatial_reference_list_any(list);
    char* text = pdal_spatial_reference_text(any);
    EXPECT_STREQ(text, srsText1);
    pdal_string_free(text);
    pdal_spatial_reference_destroy(any);

    pdal_spatial_reference_list_add(list, srs2);
    EXPECT_FALSE(pdal_spatial_reference_list_unique(list));
    EXPECT_EQ(pdal_spatial_reference_list_size(list), 2u);
    any = pdal_spatial_reference_list_any(list);
    text = pdal_spatial_reference_text(any);
    EXPECT_STREQ(text, srsText2);
    pdal_string_free(text);
    pdal_spatial_reference_destroy(any);

    pdal_spatial_reference_list_add(list, srs1);
    EXPECT_FALSE(pdal_spatial_reference_list_unique(list));
    EXPECT_EQ(pdal_spatial_reference_list_size(list), 2u);
    any = pdal_spatial_reference_list_any(list);
    text = pdal_spatial_reference_text(any);
    EXPECT_STREQ(text, srsText1);
    pdal_string_free(text);
    pdal_spatial_reference_destroy(any);

    pdal_spatial_reference_list_destroy(list);
    pdal_spatial_reference_destroy(srs2);
    pdal_spatial_reference_destroy(srs1);
}

void simpleTest()
{
    constexpr int U16 = 1;
    constexpr int F64 = 9;

    pdal_point_layout_t* layout = pdal_point_layout_create();
    pdal_point_layout_register_dim(layout, "X", F64);
    pdal_point_layout_register_dim(layout, "Y", F64);
    pdal_point_layout_register_dim(layout, "Z", F64);
    pdal_point_layout_register_dim(layout, "Intensity", U16);
    pdal_point_layout_register_dim(layout, "Blue", U16);

    pdal_point_view_t* v = pdal_point_view_create(layout);
    ASSERT_NE(v, nullptr);

    for (PointId id = 0; id < 10000; id++)
    {
        EXPECT_EQ(pdal_point_view_add_point(v), id);
        if (id % 200 < 100)
        {
            pdal_point_view_set_f64(v, id, "X", id);
            pdal_point_view_set_f64(v, id, "Y", id + 1);
            pdal_point_view_set_f64(v, id, "Z", id + 2);
            pdal_point_view_set_f64(v, id, "Intensity", (id * 100) % 6523);
        }
        else
            pdal_point_view_set_f64(v, id, "Blue", 0);
    }

    for (PointId id = 0; id < 10000; id++)
    {
        if (id % 200 < 100)
        {
            EXPECT_DOUBLE_EQ(pdal_point_view_get_f64(v, id, "X"), id);
            EXPECT_DOUBLE_EQ(pdal_point_view_get_f64(v, id, "Y"), id + 1);
            EXPECT_DOUBLE_EQ(pdal_point_view_get_f64(v, id, "Z"), id + 2);
            EXPECT_DOUBLE_EQ(pdal_point_view_get_f64(v, id, "Intensity"),
                             (id * 100) % 6523);
        }
        else
        {
            EXPECT_DOUBLE_EQ(pdal_point_view_get_f64(v, id, "X"), 0);
            EXPECT_DOUBLE_EQ(pdal_point_view_get_f64(v, id, "Y"), 0);
            EXPECT_DOUBLE_EQ(pdal_point_view_get_f64(v, id, "Z"), 0);
            EXPECT_DOUBLE_EQ(pdal_point_view_get_f64(v, id, "Intensity"), 0);
        }
    }

    pdal_point_view_destroy(v);
}

TEST(PointTable, simple)
{
    simpleTest();
}

TEST(ColumnPointTable, typedStorage)
{
    ColumnPointTable table;
    PointLayoutPtr layout = table.layout();

    Dimension::Id s8 = layout->assignDim("s8", Dimension::Type::Signed8);
    Dimension::Id s16 = layout->assignDim("s16", Dimension::Type::Signed16);
    Dimension::Id s32 = layout->assignDim("s32", Dimension::Type::Signed32);
    Dimension::Id s64 = layout->assignDim("s64", Dimension::Type::Signed64);
    Dimension::Id u8 = layout->assignDim("u8", Dimension::Type::Unsigned8);
    Dimension::Id u16 = layout->assignDim("u16", Dimension::Type::Unsigned16);
    Dimension::Id u32 = layout->assignDim("u32", Dimension::Type::Unsigned32);
    Dimension::Id u64 = layout->assignDim("u64", Dimension::Type::Unsigned64);
    Dimension::Id f32 = layout->assignDim("f32", Dimension::Type::Float);
    Dimension::Id f64 = layout->assignDim("f64", Dimension::Type::Double);
    layout->registerDim(Dimension::Id::X);
    table.finalize();

    PointView view(table);
    const PointId blockPoint = 16384;
    for (PointId id = 0; id <= blockPoint; ++id)
        view.setField(Dimension::Id::X, id, static_cast<double>(id));

    const auto setValues = [&](PointId id)
    {
        view.setField(s8, id, static_cast<int8_t>(-12));
        view.setField(s16, id, static_cast<int16_t>(-1234));
        view.setField(s32, id, static_cast<int32_t>(-123456));
        view.setField(s64, id, static_cast<int64_t>(-1234567890123LL));
        view.setField(u8, id, static_cast<uint8_t>(250));
        view.setField(u16, id, static_cast<uint16_t>(65000));
        view.setField(u32, id, static_cast<uint32_t>(4000000000U));
        view.setField(u64, id, static_cast<uint64_t>(9000000000000000000ULL));
        view.setField(f32, id, 1.25f);
        view.setField(f64, id, -9876.5);
    };

    setValues(0);
    setValues(blockPoint);

    for (PointId id : {PointId(0), blockPoint})
    {
        EXPECT_EQ(view.getFieldAs<int8_t>(s8, id), -12);
        EXPECT_EQ(view.getFieldAs<int16_t>(s16, id), -1234);
        EXPECT_EQ(view.getFieldAs<int32_t>(s32, id), -123456);
        EXPECT_EQ(view.getFieldAs<int64_t>(s64, id), -1234567890123LL);
        EXPECT_EQ(view.getFieldAs<uint8_t>(u8, id), 250u);
        EXPECT_EQ(view.getFieldAs<uint16_t>(u16, id), 65000u);
        EXPECT_EQ(view.getFieldAs<uint32_t>(u32, id), 4000000000U);
        EXPECT_EQ(view.getFieldAs<uint64_t>(u64, id), 9000000000000000000ULL);
        EXPECT_FLOAT_EQ(view.getFieldAs<float>(f32, id), 1.25f);
        EXPECT_DOUBLE_EQ(view.getFieldAs<double>(f64, id), -9876.5);
    }
}

TEST(PointTable, layoutLimit)
{
    PointTable t;
    PointLayoutPtr layout = t.layout();
    layout->setAllowedDims({"X", "Z"});

    layout->registerDim(Dimension::Id::X);
    layout->registerDim(Dimension::Id::Y);
    layout->registerDim(Dimension::Id::Z);
    layout->registerDim(Dimension::Id::Intensity);
    layout->registerDim(Dimension::Id::Blue);
    t.finalize();

    PointView v(t);
    for (PointId id = 0; id < 1000; id++)
    {
        if (id % 200 < 100)
        {
            v.setField(Dimension::Id::X, id, id);
            v.setField(Dimension::Id::Y, id, id + 1);
            v.setField(Dimension::Id::Z, id, id + 2);
            v.setField(Dimension::Id::Intensity, id, (id * 100) % 6523);
        }
        else
        {
            v.setField(Dimension::Id::X, id, 0);
            v.setField(Dimension::Id::Blue, id, id);
        }
    }

    for (PointId id = 0; id < 1000; id++)
    {
        if (id % 200 < 100)
        {
            EXPECT_EQ(id, v.getFieldAs<PointId>(Dimension::Id::X, id));
            EXPECT_EQ(id + 1, v.getFieldAs<PointId>(Dimension::Id::Y, id));
            EXPECT_EQ(id + 2, v.getFieldAs<PointId>(Dimension::Id::Z, id));
        }
        else
        {
            EXPECT_EQ(0U, v.getFieldAs<PointId>(Dimension::Id::X, id));
            EXPECT_EQ(0U, v.getFieldAs<PointId>(Dimension::Id::Y, id));
            EXPECT_EQ(0U, v.getFieldAs<PointId>(Dimension::Id::Z, id));
        }
        EXPECT_EQ(0U, v.getFieldAs<PointId>(Dimension::Id::Intensity, id));
        EXPECT_EQ(0U, v.getFieldAs<PointId>(Dimension::Id::Blue, id));
    }
}

} // namespace pdal
