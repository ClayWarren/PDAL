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

#include "FbiReader.hpp"

#include <pdal/PointView.hpp>
#include <pdal/PDALUtils.hpp>
#include <pdal/pdal_types.hpp>

#include <algorithm>
#include <array>
#include <cstring>
#include <initializer_list>
#include <limits>
#include <string>
#include <type_traits>

namespace pdal
{

namespace
{

constexpr fbi::UINT64 FbiHeaderSize = 1808;
static_assert(
    sizeof(fbi::NrmVec) == sizeof(fbi::UINT),
    "FBI normal vectors must use their 32-bit on-disk representation.");

template <typename T>
void readExact(std::istream& stream, T& value, const char* description)
{
    stream.read(reinterpret_cast<char*>(&value), sizeof(T));
    if (!stream)
        throw pdal_error("Unable to read FBI " + std::string(description) +
                         ".");
}

template <typename T> T readExact(std::istream& stream, const char* description)
{
    T value{};
    readExact(stream, value, description);
    return value;
}

void seekExact(std::istream& stream, fbi::UINT64 position,
               const char* description)
{
    const fbi::UINT64 maxPosition =
        static_cast<fbi::UINT64>((std::numeric_limits<std::streamoff>::max)());
    if (position > maxPosition)
        throw pdal_error("Invalid FBI " + std::string(description) +
                         " stream position.");

    stream.seekg(static_cast<std::streamoff>(position), std::ios::beg);
    if (!stream)
        throw pdal_error("Unable to seek to FBI " + std::string(description) +
                         " stream.");
}

fbi::UINT64 fileSize(std::istream& stream)
{
    stream.seekg(0, std::ios::end);
    if (!stream)
        throw pdal_error("Unable to determine FBI file size.");

    const std::streamoff size = stream.tellg();
    if (size < 0)
        throw pdal_error("Unable to determine FBI file size.");
    return static_cast<fbi::UINT64>(size);
}

void validateBits(const char* name, fbi::UINT bits,
                  std::initializer_list<fbi::UINT> allowed)
{
    if (std::find(allowed.begin(), allowed.end(), bits) == allowed.end())
        throw pdal_error("Invalid FBI header: unsupported " +
                         std::string(name) + " value " + std::to_string(bits) +
                         ".");
}

void validateStreamRange(const char* name, fbi::UINT64 position,
                         fbi::UINT64 count, fbi::UINT64 bytesPerPoint,
                         fbi::UINT64 headerSize, fbi::UINT64 size)
{
    if (position < headerSize || position > size)
        throw pdal_error("Invalid FBI " + std::string(name) +
                         " stream position.");

    if (count &&
        bytesPerPoint > (std::numeric_limits<fbi::UINT64>::max)() / count)
        throw pdal_error("Invalid FBI " + std::string(name) + " stream size.");

    const fbi::UINT64 byteCount = count * bytesPerPoint;
    if (byteCount > size - position)
        throw pdal_error("Truncated FBI " + std::string(name) + " stream.");
}

fbi::UINT colorComponentBytes(fbi::UINT bits)
{
    return bits == 24 || bits == 32 ? 1 : 2;
}

fbi::UINT colorComponentCount(fbi::UINT bits)
{
    return bits == 32 || bits == 64 ? 4 : 3;
}

void validateFbiHeader(const fbi::FbiHdr& hdr, fbi::UINT64 size)
{
    static const std::array<char, 8> Signature = {
        {'F', 'A', 'S', 'T', 'B', 'I', 'N', '\0'}};
    if (std::memcmp(hdr.Signature, Signature.data(), Signature.size()))
        throw pdal_error("Invalid FBI header signature.");
    if (hdr.Version != 1)
        throw pdal_error("Unsupported FBI header version " +
                         std::to_string(hdr.Version) + ".");
    if (hdr.HdrSize < FbiHeaderSize || hdr.HdrSize > size)
        throw pdal_error("Invalid FBI header size.");
    if (!hdr.UnitsXyz)
        throw pdal_error("Invalid FBI XYZ scale factor.");

    validateBits("BitsX", hdr.BitsX, {32});
    validateBits("BitsY", hdr.BitsY, {32});
    validateBits("BitsZ", hdr.BitsZ, {32});
    validateBits("BitsTime", hdr.BitsTime, {0, 64});
    validateBits("BitsDistance", hdr.BitsDistance, {0, 32});
    validateBits("BitsGroup", hdr.BitsGroup, {0, 32});
    validateBits("BitsNormal", hdr.BitsNormal, {0, 32});
    validateBits("BitsColor", hdr.BitsColor, {0, 24, 32, 48, 64});
    validateBits("BitsIntensity", hdr.BitsIntensity, {0, 16});
    validateBits("BitsLine", hdr.BitsLine, {0, 16});
    validateBits("BitsEchoLen", hdr.BitsEchoLen, {0, 16});
    validateBits("BitsAmplitude", hdr.BitsAmplitude, {0, 16});
    validateBits("BitsScanner", hdr.BitsScanner, {0, 8});
    validateBits("BitsEcho", hdr.BitsEcho, {0, 8});
    validateBits("BitsAngle", hdr.BitsAngle, {0, 8});
    validateBits("BitsEchoNorm", hdr.BitsEchoNorm, {0, 8});
    validateBits("BitsClass", hdr.BitsClass, {0, 8});
    validateBits("BitsEchoPos", hdr.BitsEchoPos, {0, 16});
    validateBits("BitsImage", hdr.BitsImage, {0, 16, 32});
    validateBits("BitsReflect", hdr.BitsReflect, {0, 16});
    validateBits("BitsDeviation", hdr.BitsDeviation, {0, 16});
    validateBits("BitsReliab", hdr.BitsReliab, {0, 8});

    validateStreamRange("XYZ", hdr.PosXyz, hdr.FastCnt, 3 * sizeof(fbi::UINT),
                        hdr.HdrSize, size);

    auto validateOptional = [&hdr, size](const char* name, fbi::UINT bits,
                                         fbi::UINT64 position,
                                         fbi::UINT64 bytesPerPoint)
    {
        if (bits)
            validateStreamRange(name, position, hdr.FastCnt, bytesPerPoint,
                                hdr.HdrSize, size);
    };

    validateOptional("time", hdr.BitsTime, hdr.PosTime, sizeof(fbi::UINT64));
    validateOptional("distance", hdr.BitsDistance, hdr.PosDistance,
                     sizeof(fbi::UINT));
    validateOptional("group", hdr.BitsGroup, hdr.PosGroup, sizeof(fbi::UINT));
    validateOptional("normal", hdr.BitsNormal, hdr.PosNormal,
                     sizeof(fbi::NrmVec));
    if (hdr.BitsColor)
        validateStreamRange("color", hdr.PosColor, hdr.FastCnt,
                            colorComponentCount(hdr.BitsColor) *
                                colorComponentBytes(hdr.BitsColor),
                            hdr.HdrSize, size);
    validateOptional("intensity", hdr.BitsIntensity, hdr.PosIntensity,
                     sizeof(uint16_t));
    validateOptional("line", hdr.BitsLine, hdr.PosLine, sizeof(uint16_t));
    validateOptional("echo length", hdr.BitsEchoLen, hdr.PosEchoLen,
                     sizeof(uint16_t));
    validateOptional("amplitude", hdr.BitsAmplitude, hdr.PosAmplitude,
                     sizeof(uint16_t));
    validateOptional("scanner", hdr.BitsScanner, hdr.PosScanner,
                     sizeof(fbi::BYTE));
    validateOptional("echo", hdr.BitsEcho, hdr.PosEcho, sizeof(fbi::BYTE));
    validateOptional("angle", hdr.BitsAngle, hdr.PosAngle, sizeof(fbi::BYTE));
    validateOptional("echo normality", hdr.BitsEchoNorm, hdr.PosEchoNorm,
                     sizeof(fbi::BYTE));
    validateOptional("classification", hdr.BitsClass, hdr.PosClass,
                     sizeof(fbi::BYTE));
    validateOptional("echo position", hdr.BitsEchoPos, hdr.PosEchoPos,
                     sizeof(uint16_t));
    validateOptional("image index", hdr.BitsImage, hdr.PosImage,
                     hdr.BitsImage / 8);
    validateOptional("reflectance", hdr.BitsReflect, hdr.PosReflect,
                     sizeof(uint16_t));
    validateOptional("deviation", hdr.BitsDeviation, hdr.PosDeviation,
                     sizeof(uint16_t));
    validateOptional("reliability", hdr.BitsReliab, hdr.PosReliab,
                     sizeof(fbi::BYTE));

    if (hdr.ImgNbrCnt && !hdr.BitsImage)
        throw pdal_error("Invalid FBI image table without image indexes.");
    if (hdr.ImgNbrCnt)
        validateStreamRange("image number", hdr.PosImgNbr, hdr.ImgNbrCnt,
                            sizeof(fbi::UINT64), hdr.HdrSize, size);
}

void readFbiHeader(fbi::FbiHdr& hdr, std::istream& stream)
{
    static_assert(sizeof(fbi::FbiHdr) == FbiHeaderSize,
                  "FBI header layout must match its on-disk representation.");
    static_assert(
        std::is_trivially_copyable<fbi::FbiHdr>::value,
        "FBI header must be safe to read from its on-disk representation.");
    readExact(stream, hdr, "header");
}

uint16_t readColorComponent(std::istream& stream, fbi::UINT bits,
                            const char* description)
{
    if (colorComponentBytes(bits) == 1)
        return readExact<fbi::BYTE>(stream, description);
    return readExact<uint16_t>(stream, description);
}

template <typename T>
void readDimension(std::istream& stream, fbi::UINT64 position,
                   point_count_t pointCount, PointViewPtr view,
                   Dimension::Id dimension, const char* description)
{
    seekExact(stream, position, description);
    for (PointId i = 0; i < pointCount; ++i)
        view->setField(dimension, i, readExact<T>(stream, description));
}

} // unnamed namespace

static StaticPluginInfo const s_info
{
    "readers.fbi",
    "Fbi Reader",
    "https://pdal.org/stages/readers.fbi.html",
    { "bin", "fbi" }
};

CREATE_STATIC_STAGE(FbiReader, s_info)

FbiReader::FbiReader()
    : pdal::Reader()
    , hdr(new fbi::FbiHdr())
    , m_istreamPtr(nullptr)
{
}

std::string FbiReader::getName() const { return s_info.name; }

void FbiReader::initialize()
{
    std::unique_ptr<std::istream> stream(Utils::openFile(m_filename, true));
    if (!stream)
        throwError("Couldn't open '" + m_filename + "'.");

    readFbiHeader(*hdr, *stream);
    validateFbiHeader(*hdr, fileSize(*stream));
    hdr->dump(log());
}

void FbiReader::addArgs(ProgramArgs& args)
{
    //nothing for now
}

void FbiReader::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Dimension::Id::X);
    layout->registerDim(Dimension::Id::Y);
    layout->registerDim(Dimension::Id::Z);

    if (hdr->BitsEcho > 0) layout->registerDim(Dimension::Id::ReturnNumber);
    if (hdr->BitsTime > 0) layout->registerDim(Dimension::Id::EchoRange);

    //Fbi assumes only uint8 for ScanAngleRank
    if (hdr->BitsAngle > 0) layout->registerDim(Dimension::Id::ScanAngleRank, Dimension::Type::Signed8);

    if (hdr->BitsClass > 0) layout->registerDim(Dimension::Id::Classification);
    if (hdr->BitsLine > 0) layout->registerDim(Dimension::Id::PointSourceId);
    if (hdr->BitsIntensity > 0) layout->registerDim(Dimension::Id::Intensity);
    if (hdr->BitsGroup > 0) layout->registerDim(Dimension::Id::ClusterID);
    if (hdr->BitsScanner > 0) layout->registerDim(Dimension::Id::UserData);
    if (hdr->BitsTime > 0) layout->registerDim(Dimension::Id::OffsetTime);
    if (hdr->BitsDistance > 0) layout->registerDim(Dimension::Id::NNDistance);
    if (hdr->BitsReliab > 0) layout->registerDim(Dimension::Id::Reliability);
    if (hdr->BitsReflect > 0) layout->registerDim(Dimension::Id::Reflectance);
    if (hdr->BitsDeviation > 0) layout->registerDim(Dimension::Id::Deviation);
    if (hdr->BitsAmplitude > 0) layout->registerDim(Dimension::Id::Amplitude);
    if (hdr->BitsEchoPos > 0) layout->registerDim(Dimension::Id::EchoPos);
    if (hdr->BitsEchoNorm > 0) layout->registerDim(Dimension::Id::EchoNorm);
    if (hdr->BitsEchoLen > 0) layout->registerDim(Dimension::Id::PulseWidth);
    if (hdr->BitsImage > 0) layout->registerDim(Dimension::Id::Image);

    if (hdr->BitsNormal > 0)
    {
        layout->registerDim(Dimension::Id::NormalX);
        layout->registerDim(Dimension::Id::NormalY);
        layout->registerDim(Dimension::Id::NormalZ);
        layout->registerDim(Dimension::Id::Dimension);
    }

    if (hdr->BitsColor > 0)
    {
        // if (hdr->BitsColor == 24) : 3 bytes of RGB (1 byte by canal)
        // if (hdr->BitsColor == 32) : 3 bytes of RGBI (1 byte by canal)
        // if (hdr->BitsColor == 48) : 3*2 bytes of RGB (2 bytes by canal)
        // if (hdr->BitsColor == 64) : 3*2 bytes of RGBI (2 bytes by canal)

        layout->registerDim(Dimension::Id::Red);
        layout->registerDim(Dimension::Id::Green);
        layout->registerDim(Dimension::Id::Blue);
        if (hdr->BitsColor == 64 || hdr->BitsColor == 32)
            layout->registerDim(Dimension::Id::Infrared);
    }
}

void FbiReader::ready(PointTableRef)
{
    m_istreamPtr.reset(Utils::openFile(m_filename, true));
    if (!m_istreamPtr)
        throwError("Couldn't open '" + m_filename + "'.");
    seekExact(*m_istreamPtr, hdr->HdrSize, "point data");
}

// Normal vector lookup tables
static int NrmTblInit=0;
static double NrmHcos[32768];
static double NrmHsin[32768];
static double NrmVsin[32768];
static double NrmVxml[32768];

// ==================================================================
// Fill lookup tables for NrmVecGetVector() routine.
// ==================================================================

void NrmVecFillLookups( void)
{
    double Hml = fbi::hc_2pi / 32767.0 ;
    double Vml = fbi::hc_pi / 32767.0 ;
    double Ang ;
    double Xml ;
    double Zvl ;

    // Fill horizontal angle tables
    for( int K(0) ; K < 32768 ; K++)
    {
        Ang = Hml * K ;
        NrmHcos[K] = cos(Ang) ;
        NrmHsin[K] = sin(Ang) ;
    }

    // Fill vertical angle tables
    for( int K(0) ; K < 32768 ; K++)
    {
        Ang = (Vml * K) - fbi::hc_piover2 ;
        Zvl = sin(Ang) ;
        Xml = sqrt( 1.0 - (Zvl * Zvl)) ;
        NrmVsin[K] = Zvl ;
        NrmVxml[K] = Xml ;
    }
}

// ==================================================================
// Get normalized direction vector from NrmVec structure.
// ==================================================================

void NrmVecGetVector( double& norm_x, double& norm_y, double& norm_z, const fbi::NrmVec *Vp)
{
    if (!NrmTblInit) {
        NrmTblInit = 1 ;
        NrmVecFillLookups() ;
    }

    int H = Vp->HorzAng ;
    int V = Vp->VertAng ;
    double Xml = NrmVxml[V] ;
    norm_x = Xml * NrmHcos[H] ;
    norm_y = Xml * NrmHsin[H] ;
    norm_z = NrmVsin[V] ;
}

point_count_t FbiReader::read(PointViewPtr view, point_count_t count)
{
    if (!m_istreamPtr)
        throwError("FBI input stream is not open.");

    const point_count_t pointCount =
        (std::min)(count, static_cast<point_count_t>(hdr->FastCnt));
    const double multiplier = 1.0 / hdr->UnitsXyz;

    seekExact(*m_istreamPtr, hdr->PosXyz, "XYZ");
    for (PointId i = 0; i < pointCount; ++i)
    {
        const fbi::UINT xr =
            readExact<fbi::UINT>(*m_istreamPtr, "X coordinate");
        const fbi::UINT yr =
            readExact<fbi::UINT>(*m_istreamPtr, "Y coordinate");
        const fbi::UINT zr =
            readExact<fbi::UINT>(*m_istreamPtr, "Z coordinate");
        view->setField(Dimension::Id::X, i, xr * multiplier + hdr->OrgX);
        view->setField(Dimension::Id::Y, i, yr * multiplier + hdr->OrgY);
        view->setField(Dimension::Id::Z, i, zr * multiplier + hdr->OrgZ);
    }

    if (hdr->BitsTime)
    {
        seekExact(*m_istreamPtr, hdr->PosTime, "time");
        for (PointId i = 0; i < pointCount; ++i)
        {
            const fbi::UINT64 time =
                readExact<fbi::UINT64>(*m_istreamPtr, "time value");
            view->setField(Dimension::Id::OffsetTime, i,
                           static_cast<uint32_t>(time));
        }
    }

    if (hdr->BitsDistance)
        readDimension<fbi::UINT>(*m_istreamPtr, hdr->PosDistance, pointCount,
                                 view, Dimension::Id::NNDistance,
                                 "distance value");

    if (hdr->BitsGroup)
        readDimension<fbi::UINT>(*m_istreamPtr, hdr->PosGroup, pointCount, view,
                                 Dimension::Id::ClusterID, "group value");

    if (hdr->BitsNormal)
    {
        seekExact(*m_istreamPtr, hdr->PosNormal, "normal");
        for (PointId i = 0; i < pointCount; ++i)
        {
            const fbi::NrmVec normal =
                readExact<fbi::NrmVec>(*m_istreamPtr, "normal value");
            double x;
            double y;
            double z;
            NrmVecGetVector(x, y, z, &normal);
            view->setField(Dimension::Id::Dimension, i,
                           static_cast<uint8_t>(normal.Dim));
            view->setField(Dimension::Id::NormalX, i, x);
            view->setField(Dimension::Id::NormalY, i, y);
            view->setField(Dimension::Id::NormalZ, i, z);
        }
    }

    if (hdr->BitsColor)
    {
        seekExact(*m_istreamPtr, hdr->PosColor, "color");
        const bool hasInfrared =
            view->layout()->hasDim(Dimension::Id::Infrared);
        for (PointId i = 0; i < pointCount; ++i)
        {
            view->setField(Dimension::Id::Red, i,
                           readColorComponent(*m_istreamPtr, hdr->BitsColor,
                                              "red color value"));
            view->setField(Dimension::Id::Green, i,
                           readColorComponent(*m_istreamPtr, hdr->BitsColor,
                                              "green color value"));
            view->setField(Dimension::Id::Blue, i,
                           readColorComponent(*m_istreamPtr, hdr->BitsColor,
                                              "blue color value"));
            if (hasInfrared)
                view->setField(Dimension::Id::Infrared, i,
                               readColorComponent(*m_istreamPtr, hdr->BitsColor,
                                                  "infrared color value"));
        }
    }

    if (hdr->BitsIntensity)
        readDimension<uint16_t>(*m_istreamPtr, hdr->PosIntensity, pointCount,
                                view, Dimension::Id::Intensity,
                                "intensity value");

    if (hdr->BitsLine)
    {
        seekExact(*m_istreamPtr, hdr->PosLine, "line");
        for (PointId i = 0; i < pointCount; ++i)
        {
            const uint16_t line =
                readExact<uint16_t>(*m_istreamPtr, "line value");
            view->setField(Dimension::Id::PointSourceId, i, line);
        }
    }

    if (hdr->BitsEchoLen)
    {
        seekExact(*m_istreamPtr, hdr->PosEchoLen, "echo length");
        for (PointId i = 0; i < pointCount; ++i)
        {
            const uint16_t length =
                readExact<uint16_t>(*m_istreamPtr, "echo length value");
            view->setField(Dimension::Id::PulseWidth, i, length);
        }
    }

    if (hdr->BitsAmplitude)
        readDimension<uint16_t>(*m_istreamPtr, hdr->PosAmplitude, pointCount,
                                view, Dimension::Id::Amplitude,
                                "amplitude value");

    if (hdr->BitsScanner)
        readDimension<fbi::BYTE>(*m_istreamPtr, hdr->PosScanner, pointCount,
                                 view, Dimension::Id::UserData,
                                 "scanner value");

    if (hdr->BitsEcho)
        readDimension<fbi::BYTE>(*m_istreamPtr, hdr->PosEcho, pointCount, view,
                                 Dimension::Id::ReturnNumber, "echo value");

    if (hdr->BitsAngle)
    {
        seekExact(*m_istreamPtr, hdr->PosAngle, "angle");
        for (PointId i = 0; i < pointCount; ++i)
        {
            const fbi::BYTE angle =
                readExact<fbi::BYTE>(*m_istreamPtr, "angle value");
            view->setField(Dimension::Id::ScanAngleRank, i,
                           static_cast<int8_t>(angle));
        }
    }

    if (hdr->BitsEchoNorm)
        readDimension<fbi::BYTE>(*m_istreamPtr, hdr->PosEchoNorm, pointCount,
                                 view, Dimension::Id::EchoNorm,
                                 "echo normality value");

    if (hdr->BitsClass)
        readDimension<fbi::BYTE>(*m_istreamPtr, hdr->PosClass, pointCount, view,
                                 Dimension::Id::Classification,
                                 "classification value");

    if (hdr->BitsEchoPos)
        readDimension<uint16_t>(*m_istreamPtr, hdr->PosEchoPos, pointCount,
                                view, Dimension::Id::EchoPos,
                                "echo position value");

    if (hdr->BitsImage)
    {
        std::vector<fbi::UINT64> imageNumbers;
        if (hdr->ImgNbrCnt)
        {
            imageNumbers.reserve(hdr->ImgNbrCnt);
            seekExact(*m_istreamPtr, hdr->PosImgNbr, "image number");
            for (fbi::UINT i = 0; i < hdr->ImgNbrCnt; ++i)
                imageNumbers.push_back(readExact<fbi::UINT64>(
                    *m_istreamPtr, "image number value"));
        }

        seekExact(*m_istreamPtr, hdr->PosImage, "image index");
        for (PointId i = 0; i < pointCount; ++i)
        {
            const fbi::UINT index =
                hdr->BitsImage == 16
                    ? readExact<uint16_t>(*m_istreamPtr, "image index value")
                    : readExact<fbi::UINT>(*m_istreamPtr, "image index value");
            fbi::UINT64 image = index;
            if (!imageNumbers.empty())
            {
                if (index >= imageNumbers.size())
                    throwError("Invalid FBI image index value.");
                image = imageNumbers[index];
            }
            if (image > (std::numeric_limits<uint16_t>::max)())
                throwError("FBI image number exceeds the supported range.");
            view->setField(Dimension::Id::Image, i,
                           static_cast<uint16_t>(image));
        }
    }

    if (hdr->BitsReflect)
        readDimension<uint16_t>(*m_istreamPtr, hdr->PosReflect, pointCount,
                                view, Dimension::Id::Reflectance,
                                "reflectance value");

    if (hdr->BitsDeviation)
        readDimension<uint16_t>(*m_istreamPtr, hdr->PosDeviation, pointCount,
                                view, Dimension::Id::Deviation,
                                "deviation value");

    if (hdr->BitsReliab)
        readDimension<fbi::BYTE>(*m_istreamPtr, hdr->PosReliab, pointCount,
                                 view, Dimension::Id::Reliability,
                                 "reliability value");

    // ToDo : read the additional points
    for (fbi::UINT64 i = 0; i < hdr->RecCnt; ++i)
    {
    }

    return pointCount;
}

void FbiReader::done(PointTableRef)
{
    m_istreamPtr.reset();
}

} // namespace pdal
