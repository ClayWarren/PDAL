/******************************************************************************
 * Copyright (c) 2026, Hobu Inc. (info@hobu.co)
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

#include <io/MemoryViewReader.hpp>

#include <cstddef>
#include <sstream>
#include <vector>

namespace pdal
{

namespace
{

struct MemoryPoint
{
    double m_x;
    double m_y;
    double m_z;
    uint16_t m_intensity;
};

} // unnamed namespace

TEST(MemoryViewReaderTest, readsFieldsFromMemory)
{
    std::vector<MemoryPoint> points{
        {1.0, 2.0, 3.0, 10}, {4.0, 5.0, 6.0, 20}, {7.0, 8.0, 9.0, 30}};

    MemoryViewReader reader;
    reader.pushField(
        {"X", Dimension::Type::Double, offsetof(MemoryPoint, m_x)});
    reader.pushField(
        {"Y", Dimension::Type::Double, offsetof(MemoryPoint, m_y)});
    reader.pushField(
        {"Z", Dimension::Type::Double, offsetof(MemoryPoint, m_z)});
    reader.pushField({"Intensity", Dimension::Type::Unsigned16,
                      offsetof(MemoryPoint, m_intensity)});
    reader.setIncrementer(
        [&points](PointId id) -> char*
        {
            if (id >= points.size())
                return nullptr;
            return reinterpret_cast<char*>(&points[id]);
        });

    PointTable table;
    reader.prepare(table);
    PointViewSet views = reader.execute(table);

    ASSERT_EQ(views.size(), 1u);
    PointViewPtr view = *views.begin();
    ASSERT_EQ(view->size(), points.size());

    for (PointId id = 0; id < points.size(); ++id)
    {
        EXPECT_DOUBLE_EQ(view->getFieldAs<double>(Dimension::Id::X, id),
                         points[id].m_x);
        EXPECT_DOUBLE_EQ(view->getFieldAs<double>(Dimension::Id::Y, id),
                         points[id].m_y);
        EXPECT_DOUBLE_EQ(view->getFieldAs<double>(Dimension::Id::Z, id),
                         points[id].m_z);
        EXPECT_EQ(view->getFieldAs<uint16_t>(Dimension::Id::Intensity, id),
                  points[id].m_intensity);
    }
}

TEST(MemoryViewReaderTest, rejectsMalformedShape)
{
    MemoryViewReader::Shape shape;

    std::istringstream empty("");
    EXPECT_THROW(empty >> shape, pdal_error);

    std::istringstream tooShort("1, 2");
    EXPECT_THROW(tooShort >> shape, pdal_error);

    std::istringstream tooLong("1, 2, 3, 4");
    EXPECT_THROW(tooLong >> shape, pdal_error);
}

TEST(MemoryViewReaderTest, synthesizesRowMajorShapeCoordinates)
{
    std::vector<uint16_t> values{10, 20, 30, 40, 50, 60};

    MemoryViewReader reader;
    Options options;
    options.add("shape", "1, 2, 3");
    reader.setOptions(options);
    reader.pushField({"Intensity", Dimension::Type::Unsigned16, 0});
    reader.setIncrementer(
        [&values](PointId id) -> char*
        {
            if (id >= values.size())
                return nullptr;
            return reinterpret_cast<char*>(&values[id]);
        });

    PointTable table;
    reader.prepare(table);
    PointViewSet views = reader.execute(table);

    ASSERT_EQ(views.size(), 1u);
    PointViewPtr view = *views.begin();
    ASSERT_EQ(view->size(), values.size());
    for (PointId id = 0; id < values.size(); ++id)
    {
        EXPECT_DOUBLE_EQ(view->getFieldAs<double>(Dimension::Id::X, id),
                         id % 3);
        EXPECT_DOUBLE_EQ(view->getFieldAs<double>(Dimension::Id::Y, id),
                         id / 3);
        EXPECT_DOUBLE_EQ(view->getFieldAs<double>(Dimension::Id::Z, id), 0.0);
        EXPECT_EQ(view->getFieldAs<uint16_t>(Dimension::Id::Intensity, id),
                  values[id]);
    }
}

} // namespace pdal
