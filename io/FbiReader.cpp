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

#include <pdal/PDALUtils.hpp>
#include <pdal/PointView.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{"readers.fbi",
                                     "Fbi Reader",
                                     "https://pdal.org/stages/readers.fbi.html",
                                     {"bin", "fbi"}};

CREATE_STATIC_STAGE(FbiReader, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

void readFbiHeader(fbi::FbiHdr* hdr, std::istream* m_istreamPtr)
{
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->Signature),
                       sizeof(hdr->Signature));
    assert(std::string(hdr->Signature) == "FASTBIN");

    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->Version),
                       sizeof(hdr->Version));
    assert(hdr->Version == 1);

    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->HdrSize),
                       sizeof(hdr->HdrSize));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->TimeType),
                       sizeof(hdr->TimeType));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->Order),
                       sizeof(hdr->Order));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->Reserved1),
                       sizeof(hdr->Reserved1));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->VlrCnt),
                       sizeof(hdr->VlrCnt));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->VlrSize),
                       sizeof(hdr->VlrSize));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->RecSize),
                       sizeof(hdr->RecSize));

    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->FastCnt),
                       sizeof(hdr->FastCnt));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->RecCnt),
                       sizeof(hdr->RecCnt));

    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->UnitsXyz),
                       sizeof(hdr->UnitsXyz));
    assert(hdr->UnitsXyz > 0.);

    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->UnitsDistance),
                       sizeof(hdr->UnitsDistance));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->OrgX), sizeof(hdr->OrgX));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->OrgY), sizeof(hdr->OrgY));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->OrgZ), sizeof(hdr->OrgZ));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->MinX), sizeof(hdr->MinX));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->MaxX), sizeof(hdr->MaxX));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->MinY), sizeof(hdr->MinY));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->MaxY), sizeof(hdr->MaxY));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->MinZ), sizeof(hdr->MinZ));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->MaxZ), sizeof(hdr->MaxZ));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->System),
                       sizeof(hdr->System));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->Software),
                       sizeof(hdr->Software));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->Reserved2),
                       sizeof(hdr->Reserved2));

    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsX),
                       sizeof(hdr->BitsX));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsY),
                       sizeof(hdr->BitsY));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsZ),
                       sizeof(hdr->BitsZ));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsTime),
                       sizeof(hdr->BitsTime));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsDistance),
                       sizeof(hdr->BitsDistance));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsGroup),
                       sizeof(hdr->BitsGroup));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsNormal),
                       sizeof(hdr->BitsNormal));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsColor),
                       sizeof(hdr->BitsColor));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsIntensity),
                       sizeof(hdr->BitsIntensity));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsLine),
                       sizeof(hdr->BitsLine));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsEchoLen),
                       sizeof(hdr->BitsEchoLen));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsAmplitude),
                       sizeof(hdr->BitsAmplitude));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsScanner),
                       sizeof(hdr->BitsScanner));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsEcho),
                       sizeof(hdr->BitsEcho));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsAngle),
                       sizeof(hdr->BitsAngle));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsEchoNorm),
                       sizeof(hdr->BitsEchoNorm));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsClass),
                       sizeof(hdr->BitsClass));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsEchoPos),
                       sizeof(hdr->BitsEchoPos));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsImage),
                       sizeof(hdr->BitsImage));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsReflect),
                       sizeof(hdr->BitsReflect));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsDeviation),
                       sizeof(hdr->BitsDeviation));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->BitsReliab),
                       sizeof(hdr->BitsReliab));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->Reserved5),
                       sizeof(hdr->Reserved5));

    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosVlr),
                       sizeof(hdr->PosVlr));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosXyz),
                       sizeof(hdr->PosXyz));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosTime),
                       sizeof(hdr->PosTime));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosDistance),
                       sizeof(hdr->PosDistance));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosGroup),
                       sizeof(hdr->PosGroup));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosNormal),
                       sizeof(hdr->PosNormal));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosColor),
                       sizeof(hdr->PosColor));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosIntensity),
                       sizeof(hdr->PosIntensity));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosLine),
                       sizeof(hdr->PosLine));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosEchoLen),
                       sizeof(hdr->PosEchoLen));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosAmplitude),
                       sizeof(hdr->PosAmplitude));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosScanner),
                       sizeof(hdr->PosScanner));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosEcho),
                       sizeof(hdr->PosEcho));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosAngle),
                       sizeof(hdr->PosAngle));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosEchoNorm),
                       sizeof(hdr->PosEchoNorm));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosClass),
                       sizeof(hdr->PosClass));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosRecord),
                       sizeof(hdr->PosRecord));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosEchoPos),
                       sizeof(hdr->PosEchoPos));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosImage),
                       sizeof(hdr->PosImage));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosReflect),
                       sizeof(hdr->PosReflect));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosDeviation),
                       sizeof(hdr->PosDeviation));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosReliab),
                       sizeof(hdr->PosReliab));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->PosImgNbr),
                       sizeof(hdr->PosImgNbr));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->ImgNbrCnt),
                       sizeof(hdr->ImgNbrCnt));
    m_istreamPtr->read(reinterpret_cast<char*>(&hdr->Reserved6),
                       sizeof(hdr->Reserved6));
}

FbiReader::FbiReader() : pdal::Reader(), hdr(new fbi::FbiHdr()) {}

FbiReader::~FbiReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string FbiReader::getName() const
{
    return s_info.name;
}

void FbiReader::initialize()
{
    m_istreamPtr = Utils::openFile(m_filename, true);
    if (!m_istreamPtr)
        throwError("Couldn't open '" + m_filename + "'.");

    m_istreamPtr->seekg(0);

    // read the hdr file
    readFbiHeader(hdr.get(), m_istreamPtr);
    hdr->dump(log());

    Utils::closeFile(m_istreamPtr);
}

void FbiReader::addArgs(ProgramArgs& args)
{
    // nothing for now
}

void FbiReader::addDimensions(PointLayoutPtr layout)
{
    m_dims = {Dimension::Id::X, Dimension::Id::Y, Dimension::Id::Z};

    if (hdr->BitsEcho > 0)
        m_dims.push_back(Dimension::Id::ReturnNumber);
    if (hdr->BitsTime > 0)
        m_dims.push_back(Dimension::Id::EchoRange);

    // Fbi assumes only uint8 for ScanAngleRank
    if (hdr->BitsAngle > 0)
    {
        layout->registerDim(Dimension::Id::ScanAngleRank,
                            Dimension::Type::Signed8);
        m_dims.push_back(Dimension::Id::ScanAngleRank);
    }
    if (hdr->BitsClass > 0)
        m_dims.push_back(Dimension::Id::Classification);
    if (hdr->BitsLine > 0)
        m_dims.push_back(Dimension::Id::PointSourceId);
    if (hdr->BitsIntensity > 0)
        m_dims.push_back(Dimension::Id::Intensity);
    if (hdr->BitsGroup > 0)
        m_dims.push_back(Dimension::Id::ClusterID);
    if (hdr->BitsScanner > 0)
        m_dims.push_back(Dimension::Id::UserData);
    if (hdr->BitsTime > 0)
        m_dims.push_back(Dimension::Id::OffsetTime);
    if (hdr->BitsDistance > 0)
        m_dims.push_back(Dimension::Id::NNDistance);
    if (hdr->BitsReliab > 0)
        m_dims.push_back(Dimension::Id::Reliability);
    if (hdr->BitsReflect > 0)
        m_dims.push_back(Dimension::Id::Reflectance);
    if (hdr->BitsDeviation > 0)
        m_dims.push_back(Dimension::Id::Deviation);
    if (hdr->BitsAmplitude > 0)
        m_dims.push_back(Dimension::Id::Amplitude);
    if (hdr->BitsEchoPos > 0)
        m_dims.push_back(Dimension::Id::EchoPos);
    if (hdr->BitsEchoNorm > 0)
        m_dims.push_back(Dimension::Id::EchoNorm);
    if (hdr->BitsEchoLen > 0)
        m_dims.push_back(Dimension::Id::PulseWidth);
    if (hdr->BitsImage > 0)
        m_dims.push_back(Dimension::Id::Image);

    if (hdr->BitsNormal > 0)
    {
        m_dims.push_back(Dimension::Id::NormalX);
        m_dims.push_back(Dimension::Id::NormalY);
        m_dims.push_back(Dimension::Id::NormalZ);
        m_dims.push_back(Dimension::Id::Dimension);
    }

    if (hdr->BitsColor > 0)
    {
        // if (hdr->BitsColor == 24) : 3 bytes of RGB (1 byte by canal)
        // if (hdr->BitsColor == 32) : 3 bytes of RGBI (1 byte by canal)
        // if (hdr->BitsColor == 48) : 3*2 bytes of RGB (2 bytes by canal)
        // if (hdr->BitsColor == 64) : 3*2 bytes of RGBI (2 bytes by canal)

        m_dims.push_back(Dimension::Id::Red);
        m_dims.push_back(Dimension::Id::Green);
        m_dims.push_back(Dimension::Id::Blue);
        if (hdr->BitsColor == 64 || hdr->BitsColor == 32)
            m_dims.push_back(Dimension::Id::Infrared);
    }

    for (Dimension::Id dim : m_dims)
    {
        if (dim == Dimension::Id::ScanAngleRank)
            continue;
        layout->registerDim(dim);
    }
}

void FbiReader::ready(PointTableRef)
{
    m_rustIndex = 0;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);

    pdal_reader_t* reader = pdal_reader_create_fbi(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust FBI reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust FBI reader failed.");
}

point_count_t FbiReader::read(PointViewPtr view, point_count_t count)
{
    point_count_t numRead = 0;
    PointId nextId = view->size();
    while (numRead < count && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        copyPoint(view, nextId);
        if (m_cb)
            m_cb(*view, nextId);

        nextId++;
        m_rustIndex++;
        numRead++;
    }

    return numRead;
}

void FbiReader::copyPoint(PointViewPtr view, PointId outIdx)
{
    for (Dimension::Id dim : m_dims)
    {
        view->setField(dim, outIdx,
                       pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                               Dimension::name(dim).c_str()));
    }
}

void FbiReader::done(PointTableRef) {}

} // namespace pdal
