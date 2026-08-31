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

#include <pdal/StageFactory.hpp>
#include <io/FbiReader.hpp>
#include "Support.hpp"

#include <array>
#include <cstddef>
#include <fstream>
#include <stdexcept>
#include <vector>

namespace pdal
{


namespace
{

std::string getTestfilePath()
{
    return Support::datapath("fbi/1.2-with-color.fbi");
}

void copyTestFile(const std::string& destination)
{
    std::ifstream input(getTestfilePath(), std::ios::binary);
    std::ofstream output(destination, std::ios::binary);
    output << input.rdbuf();
    if (!input || !output)
        throw std::runtime_error("Unable to copy FBI test file.");
}

void copyTestFilePrefix(const std::string& destination, size_t size)
{
    std::ifstream input(getTestfilePath(), std::ios::binary);
    std::ofstream output(destination, std::ios::binary);
    std::vector<char> bytes(size);
    input.read(bytes.data(), static_cast<std::streamsize>(bytes.size()));
    output.write(bytes.data(), input.gcount());
    if (input.gcount() != static_cast<std::streamsize>(bytes.size()) || !output)
        throw std::runtime_error("Unable to truncate FBI test file.");
}

template <typename T>
void patchHeader(const std::string& filename, size_t position, T value)
{
    std::fstream stream(filename,
                        std::ios::in | std::ios::out | std::ios::binary);
    stream.seekp(static_cast<std::streamoff>(position), std::ios::beg);
    stream.write(reinterpret_cast<const char*>(&value), sizeof(value));
    if (!stream)
        throw std::runtime_error("Unable to patch FBI test header.");
}

void setFilename(FbiReader& reader, const std::string& filename)
{
    Options options;
    options.add("filename", filename);
    reader.setOptions(options);
}

fbi::UINT64 appendColor48Stream(const std::string& filename,
                                fbi::UINT64 pointCount)
{
    std::ofstream stream(filename, std::ios::binary | std::ios::app);
    const fbi::UINT64 position = static_cast<fbi::UINT64>(stream.tellp());
    for (fbi::UINT64 i = 0; i < pointCount; ++i)
    {
        const uint16_t red = 1000;
        const uint16_t green = 2000;
        const uint16_t blue = 3000;
        stream.write(reinterpret_cast<const char*>(&red), sizeof(red));
        stream.write(reinterpret_cast<const char*>(&green), sizeof(green));
        stream.write(reinterpret_cast<const char*>(&blue), sizeof(blue));
    }
    if (!stream)
        throw std::runtime_error("Unable to append FBI color stream.");
    return position;
}

fbi::UINT64 appendUint16Stream(const std::string& filename,
                               fbi::UINT64 pointCount, uint16_t value)
{
    std::ofstream stream(filename, std::ios::binary | std::ios::app);
    const fbi::UINT64 position = static_cast<fbi::UINT64>(stream.tellp());
    for (fbi::UINT64 i = 0; i < pointCount; ++i)
        stream.write(reinterpret_cast<const char*>(&value), sizeof(value));
    if (!stream)
        throw std::runtime_error("Unable to append FBI 16-bit stream.");
    return position;
}

struct ImageStreams
{
    fbi::UINT64 indexPosition;
    fbi::UINT64 numberPosition;
};

ImageStreams appendImageStreams(const std::string& filename,
                                fbi::UINT64 pointCount, uint16_t firstIndex)
{
    std::ofstream stream(filename, std::ios::binary | std::ios::app);
    ImageStreams positions;
    positions.indexPosition = static_cast<fbi::UINT64>(stream.tellp());
    for (fbi::UINT64 i = 0; i < pointCount; ++i)
    {
        const uint16_t index = i ? 0 : firstIndex;
        stream.write(reinterpret_cast<const char*>(&index), sizeof(index));
    }
    positions.numberPosition = static_cast<fbi::UINT64>(stream.tellp());
    const fbi::UINT64 imageNumber = 42;
    stream.write(reinterpret_cast<const char*>(&imageNumber),
                 sizeof(imageNumber));
    if (!stream)
        throw std::runtime_error("Unable to append FBI image streams.");
    return positions;
}

class FbiReaderTest : public ::testing::Test
{
public:
    FbiReaderTest()
        : ::testing::Test()
        , m_reader()
    {
        Options options;
        options.add("filename", getTestfilePath());
        m_reader.setOptions(options);
    }

    FbiReader m_reader;
};
}

TEST_F(FbiReaderTest, Constructor)
{
    FbiReader reader1;

    StageFactory f;
    Stage* reader2(f.createStage("readers.fbi"));
}

TEST_F(FbiReaderTest, Header)
{
    PointTable table;
    m_reader.prepare(table);
    fbi::FbiHdr header = m_reader.getHeader();

    EXPECT_EQ(1808u, header.HdrSize);
    EXPECT_EQ(1u, header.Version);
    EXPECT_EQ(1065u, header.FastCnt);
    EXPECT_EQ(1808u, header.PosXyz);

    //could add more test values
}

TEST_F(FbiReaderTest, ReadingPoints)
{
    PointTable table;
    m_reader.prepare(table);
    PointViewSet viewSet = m_reader.execute(table);
    EXPECT_EQ(viewSet.size(), 1u);

    //number of points
    PointViewPtr view = *viewSet.begin();
    EXPECT_EQ(view->size(), 1065u);

    //some tests on the first point
    EXPECT_NEAR(635618.98, view->getFieldAs<double>(Dimension::Id::X, 0),1e-4);
    EXPECT_NEAR(848898.71, view->getFieldAs<double>(Dimension::Id::Y, 0),1e-4);
    EXPECT_NEAR(405.59, view->getFieldAs<double>(Dimension::Id::Z, 0),1e-4);
    EXPECT_DOUBLE_EQ(0, view->getFieldAs<double>(Dimension::Id::OffsetTime, 0));
    EXPECT_EQ(55040, view->getFieldAs<uint16_t>(Dimension::Id::Intensity, 0));
    EXPECT_EQ(45056,
              view->getFieldAs<uint16_t>(Dimension::Id::PointSourceId, 0));
    EXPECT_EQ(1, view->getFieldAs<uint8_t>(Dimension::Id::ReturnNumber, 0));
    EXPECT_EQ(0, view->getFieldAs<uint8_t>(Dimension::Id::NumberOfReturns, 0));
    EXPECT_EQ(20, view->getFieldAs<uint8_t>(Dimension::Id::Classification, 0));
}

TEST(FbiReaderValidationTest, rejectsUnsupportedBitWidths)
{
    const std::array<size_t, 22> offsets = {
        offsetof(fbi::FbiHdr, BitsX),
        offsetof(fbi::FbiHdr, BitsY),
        offsetof(fbi::FbiHdr, BitsZ),
        offsetof(fbi::FbiHdr, BitsTime),
        offsetof(fbi::FbiHdr, BitsDistance),
        offsetof(fbi::FbiHdr, BitsGroup),
        offsetof(fbi::FbiHdr, BitsNormal),
        offsetof(fbi::FbiHdr, BitsColor),
        offsetof(fbi::FbiHdr, BitsIntensity),
        offsetof(fbi::FbiHdr, BitsLine),
        offsetof(fbi::FbiHdr, BitsEchoLen),
        offsetof(fbi::FbiHdr, BitsAmplitude),
        offsetof(fbi::FbiHdr, BitsScanner),
        offsetof(fbi::FbiHdr, BitsEcho),
        offsetof(fbi::FbiHdr, BitsAngle),
        offsetof(fbi::FbiHdr, BitsEchoNorm),
        offsetof(fbi::FbiHdr, BitsClass),
        offsetof(fbi::FbiHdr, BitsEchoPos),
        offsetof(fbi::FbiHdr, BitsImage),
        offsetof(fbi::FbiHdr, BitsReflect),
        offsetof(fbi::FbiHdr, BitsDeviation),
        offsetof(fbi::FbiHdr, BitsReliab)};

    for (size_t offset : offsets)
    {
        SCOPED_TRACE(offset);
        Support::Tempfile file;
        copyTestFile(file.filename());
        patchHeader(file.filename(), offset, fbi::UINT(7));

        FbiReader reader;
        setFilename(reader, file.filename());
        PointTable table;
        EXPECT_THROW(reader.prepare(table), pdal_error);
    }
}

TEST(FbiReaderValidationTest, rejectsTruncatedStream)
{
    Support::Tempfile file;
    constexpr size_t TruncatedXyzSize = 1808 + 1065 * 3 * 4 - 1;
    copyTestFilePrefix(file.filename(), TruncatedXyzSize);

    FbiReader reader;
    setFilename(reader, file.filename());
    PointTable table;
    EXPECT_THROW(reader.prepare(table), pdal_error);
}

TEST(FbiReaderValidationTest, preserves16BitFields)
{
    Support::Tempfile file;
    copyTestFile(file.filename());
    const fbi::UINT64 linePosition =
        appendUint16Stream(file.filename(), 1065, 321);
    const fbi::UINT64 echoLengthPosition =
        appendUint16Stream(file.filename(), 1065, 654);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, BitsLine),
                fbi::UINT(16));
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, PosLine), linePosition);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, BitsEchoLen),
                fbi::UINT(16));
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, PosEchoLen),
                echoLengthPosition);

    FbiReader reader;
    setFilename(reader, file.filename());
    PointTable table;
    reader.prepare(table);
    PointViewSet viewSet = reader.execute(table);
    PointViewPtr view = *viewSet.begin();
    EXPECT_EQ(view->getFieldAs<uint16_t>(Dimension::Id::PointSourceId, 0),
              321);
    EXPECT_FLOAT_EQ(view->getFieldAs<float>(Dimension::Id::PulseWidth, 0),
                    654.0f);
}

TEST(FbiReaderValidationTest, reads48BitColorComponents)
{
    Support::Tempfile file;
    copyTestFile(file.filename());
    const fbi::UINT64 position = appendColor48Stream(file.filename(), 1065);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, BitsColor),
                fbi::UINT(48));
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, PosColor), position);

    FbiReader reader;
    setFilename(reader, file.filename());
    PointTable table;
    reader.prepare(table);
    PointViewSet viewSet = reader.execute(table);
    PointViewPtr view = *viewSet.begin();
    EXPECT_EQ(view->getFieldAs<uint16_t>(Dimension::Id::Red, 0), 1000);
    EXPECT_EQ(view->getFieldAs<uint16_t>(Dimension::Id::Green, 0), 2000);
    EXPECT_EQ(view->getFieldAs<uint16_t>(Dimension::Id::Blue, 0), 3000);
}

TEST(FbiReaderValidationTest, readsReflectanceStream)
{
    Support::Tempfile file;
    copyTestFile(file.filename());
    const fbi::UINT64 position = appendUint16Stream(file.filename(), 1065, 321);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, BitsReflect),
                fbi::UINT(16));
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, PosReflect), position);

    FbiReader reader;
    setFilename(reader, file.filename());
    PointTable table;
    reader.prepare(table);
    PointViewSet viewSet = reader.execute(table);
    PointViewPtr view = *viewSet.begin();
    EXPECT_FLOAT_EQ(view->getFieldAs<float>(Dimension::Id::Reflectance, 0),
                    321.0f);
}

TEST(FbiReaderValidationTest, resolvesImageIndexes)
{
    Support::Tempfile file;
    copyTestFile(file.filename());
    const ImageStreams positions = appendImageStreams(file.filename(), 1065, 0);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, BitsImage),
                fbi::UINT(16));
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, PosImage),
                positions.indexPosition);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, PosImgNbr),
                positions.numberPosition);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, ImgNbrCnt),
                fbi::UINT(1));

    FbiReader reader;
    setFilename(reader, file.filename());
    PointTable table;
    reader.prepare(table);
    PointViewSet viewSet = reader.execute(table);
    PointViewPtr view = *viewSet.begin();
    EXPECT_EQ(view->getFieldAs<uint16_t>(Dimension::Id::Image, 0), 42);
}

TEST(FbiReaderValidationTest, rejectsOutOfRangeImageIndex)
{
    Support::Tempfile file;
    copyTestFile(file.filename());
    const ImageStreams positions = appendImageStreams(file.filename(), 1065, 1);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, BitsImage),
                fbi::UINT(16));
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, PosImage),
                positions.indexPosition);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, PosImgNbr),
                positions.numberPosition);
    patchHeader(file.filename(), offsetof(fbi::FbiHdr, ImgNbrCnt),
                fbi::UINT(1));

    FbiReader reader;
    setFilename(reader, file.filename());
    PointTable table;
    reader.prepare(table);
    EXPECT_THROW(reader.execute(table), pdal_error);
}
}
