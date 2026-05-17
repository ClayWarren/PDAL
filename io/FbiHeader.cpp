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

#include "FbiHeader.hpp"

namespace pdal
{

namespace fbi
{

void FbiHdr::dump(const LogPtr& log)
{
    log->get(LogLevel::Debug) << "Fbi header : Signature " << Signature << '\n';
    log->get(LogLevel::Debug) << "Fbi header : Version " << Version << '\n';
    log->get(LogLevel::Debug) << "Fbi header : HdrSize " << HdrSize << '\n';
    log->get(LogLevel::Debug) << "Fbi header : TimeType " << TimeType << '\n';
    log->get(LogLevel::Debug) << "Fbi header : Order " << Order << '\n';
    log->get(LogLevel::Debug) << "Fbi header : Reserved1 " << Reserved1 << '\n';
    log->get(LogLevel::Debug) << "Fbi header : VlrCnt " << VlrCnt << '\n';
    log->get(LogLevel::Debug) << "Fbi header : VlrSize " << VlrSize << '\n';
    log->get(LogLevel::Debug) << "Fbi header : RecSize " << RecSize << '\n';
    log->get(LogLevel::Debug) << "Fbi header : FastCnt " << FastCnt << '\n';
    log->get(LogLevel::Debug) << "Fbi header : RecCnt " << RecCnt << '\n';
    log->get(LogLevel::Debug) << "Fbi header : UnitsXyz " << UnitsXyz << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : UnitsDistance " << UnitsDistance << '\n';
    log->get(LogLevel::Debug) << "Fbi header : OrgX " << OrgX << '\n';
    log->get(LogLevel::Debug) << "Fbi header : OrgY " << OrgY << '\n';
    log->get(LogLevel::Debug) << "Fbi header : OrgZ " << OrgZ << '\n';
    log->get(LogLevel::Debug) << "Fbi header : MinX " << MinX << '\n';
    log->get(LogLevel::Debug) << "Fbi header : MaxX " << MaxX << '\n';
    log->get(LogLevel::Debug) << "Fbi header : MinY " << MinY << '\n';
    log->get(LogLevel::Debug) << "Fbi header : MaxY " << MaxY << '\n';
    log->get(LogLevel::Debug) << "Fbi header : MinZ " << MinZ << '\n';
    log->get(LogLevel::Debug) << "Fbi header : MaxZ " << MaxZ << '\n';
    log->get(LogLevel::Debug) << "Fbi header : System " << System << '\n';
    log->get(LogLevel::Debug) << "Fbi header : Software " << Software << '\n';
    log->get(LogLevel::Debug) << "Fbi header : Reserved2 " << Reserved2 << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsX " << BitsX << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsY " << BitsY << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsZ " << BitsZ << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsTime " << BitsTime << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsDistance " << BitsDistance << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsGroup " << BitsGroup << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsImage " << BitsImage << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsNormal " << BitsNormal << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsColor " << BitsColor << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsIntensity " << BitsIntensity << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsLine " << BitsLine << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsEchoLen " << BitsEchoLen << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsAmplitude " << BitsAmplitude << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsScanner " << BitsScanner << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsEcho " << BitsEcho << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsAngle " << BitsAngle << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsEchoNorm " << BitsEchoNorm << '\n';
    log->get(LogLevel::Debug) << "Fbi header : BitsClass " << BitsClass << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsEchoPos " << BitsEchoPos << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsReflect " << BitsReflect << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsDeviation " << BitsDeviation << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : BitsReliab " << BitsReliab << '\n';
    log->get(LogLevel::Debug) << "Fbi header : Reserved5 " << Reserved5 << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosVlr " << PosVlr << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosXyz " << PosXyz << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosTime " << PosTime << " " << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosDistance " << PosDistance << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosGroup " << PosGroup << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosNormal " << PosNormal << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosColor " << PosColor << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosIntensity " << PosIntensity << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosLine " << PosLine << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosEchoLen " << PosEchoLen << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosAmplitude " << PosAmplitude << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosScanner " << PosScanner << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosEcho " << PosEcho << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosAngle " << PosAngle << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosEchoNorm " << PosEchoNorm << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosClass " << PosClass << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosRecord " << PosRecord << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosEchoPos " << PosEchoPos << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosImage " << PosImage << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosReflect " << PosReflect << '\n';
    log->get(LogLevel::Debug)
        << "Fbi header : PosDeviatio " << PosDeviation << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosReliab " << PosReliab << '\n';
    log->get(LogLevel::Debug) << "Fbi header : PosImgNbr " << PosImgNbr << '\n';
    log->get(LogLevel::Debug) << "Fbi header : ImgNbrCnt " << ImgNbrCnt << '\n';
    log->get(LogLevel::Debug) << "Fbi header : Reserved6 " << Reserved6 << '\n';
}

} // namespace fbi
} // namespace pdal
