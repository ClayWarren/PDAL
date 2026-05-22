/******************************************************************************
 * Copyright (c) 2021, Antoine Lavenant, antoine.lavenant@ign.fr
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
#include <io/BufferReader.hpp>
#include <io/FbiReader.hpp>
#include <io/FbiWriter.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal/util/FileUtils.hpp>

namespace pdal
{

namespace
{

std::string getTestfilePath()
{
    return Support::datapath("fbi/1.2-with-color.fbi");
}

class FbiReaderTest : public ::testing::Test
{
public:
    FbiReaderTest() : ::testing::Test(), m_reader()
    {
        Options options;
        options.add("filename", getTestfilePath());
        m_reader.setOptions(options);
    }

    FbiReader m_reader;
};
} // namespace

TEST_F(FbiReaderTest, Constructor)
{
    PointTable table;
    m_reader.prepare(table);
    EXPECT_EQ(m_reader.getName(), "readers.fbi");
}

TEST_F(FbiReaderTest, Header)
{
    PointTable table;
    m_reader.prepare(table);
    fbi::FbiHdr header = m_reader.getHeader();

    EXPECT_EQ(1808, header.HdrSize);
    EXPECT_EQ(1, header.Version);
    EXPECT_EQ(1065, header.FastCnt);
    EXPECT_EQ(1808, header.PosXyz);

    // could add more test values
}

TEST_F(FbiReaderTest, ReadingPoints)
{
    PointTable table;
    m_reader.prepare(table);
    PointViewSet viewSet = m_reader.execute(table);
    EXPECT_EQ(viewSet.size(), 1u);

    // number of points
    PointViewPtr view = *viewSet.begin();
    EXPECT_EQ(view->size(), 1065);

    // some tests on the first point
    EXPECT_NEAR(635618.98, view->getFieldAs<double>(Dimension::Id::X, 0), 1e-4);
    EXPECT_NEAR(848898.71, view->getFieldAs<double>(Dimension::Id::Y, 0), 1e-4);
    EXPECT_NEAR(405.59, view->getFieldAs<double>(Dimension::Id::Z, 0), 1e-4);
    EXPECT_DOUBLE_EQ(0, view->getFieldAs<double>(Dimension::Id::OffsetTime, 0));
    EXPECT_EQ(55040, view->getFieldAs<uint16_t>(Dimension::Id::Intensity, 0));
    EXPECT_EQ(0, view->getFieldAs<uint16_t>(Dimension::Id::PointSourceId, 0));
    EXPECT_EQ(1, view->getFieldAs<uint8_t>(Dimension::Id::ReturnNumber, 0));
    EXPECT_EQ(0, view->getFieldAs<uint8_t>(Dimension::Id::NumberOfReturns, 0));
    EXPECT_EQ(20, view->getFieldAs<uint8_t>(Dimension::Id::Classification, 0));
}

TEST(FbiWriterTest, RoundtripBasicDimensions)
{
    std::string outfile(Support::temppath("roundtrip.fbi"));
    FileUtils::deleteFile(outfile);

    PointTable table;
    table.layout()->registerDim(Dimension::Id::X);
    table.layout()->registerDim(Dimension::Id::Y);
    table.layout()->registerDim(Dimension::Id::Z);
    table.layout()->registerDim(Dimension::Id::Intensity);
    table.layout()->registerDim(Dimension::Id::Classification);

    PointViewPtr view(new PointView(table));
    view->setField(Dimension::Id::X, 0, 100.0);
    view->setField(Dimension::Id::Y, 0, 200.0);
    view->setField(Dimension::Id::Z, 0, 300.0);
    view->setField(Dimension::Id::Intensity, 0, 42);
    view->setField(Dimension::Id::Classification, 0, 7);
    view->setField(Dimension::Id::X, 1, 101.0);
    view->setField(Dimension::Id::Y, 1, 201.0);
    view->setField(Dimension::Id::Z, 1, 301.0);
    view->setField(Dimension::Id::Intensity, 1, 43);
    view->setField(Dimension::Id::Classification, 1, 8);

    BufferReader source;
    source.addView(view);

    FbiWriter writer;
    Options writerOptions;
    writerOptions.add("filename", outfile);
    writer.setOptions(writerOptions);
    writer.setInput(source);
    writer.prepare(table);
    writer.execute(table);

    FbiReader reader;
    Options readerOptions;
    readerOptions.add("filename", outfile);
    reader.setOptions(readerOptions);
    PointTable readTable;
    reader.prepare(readTable);
    PointViewSet viewSet = reader.execute(readTable);

    ASSERT_EQ(viewSet.size(), 1u);
    PointViewPtr roundtrip = *viewSet.begin();
    ASSERT_EQ(roundtrip->size(), 2u);
    EXPECT_NEAR(roundtrip->getFieldAs<double>(Dimension::Id::X, 0), 100.0,
                0.01);
    EXPECT_NEAR(roundtrip->getFieldAs<double>(Dimension::Id::Y, 0), 200.0,
                0.01);
    EXPECT_NEAR(roundtrip->getFieldAs<double>(Dimension::Id::Z, 0), 300.0,
                0.01);
    EXPECT_EQ(roundtrip->getFieldAs<uint16_t>(Dimension::Id::Intensity, 0), 42);
    EXPECT_EQ(roundtrip->getFieldAs<uint8_t>(Dimension::Id::Classification, 1),
              8);
}
} // namespace pdal
