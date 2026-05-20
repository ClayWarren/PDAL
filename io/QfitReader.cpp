/******************************************************************************
 * Copyright (c) 2011, Howard Butler, hobu.inc@gmail.com
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

/*

Airborne Topographic Mapper (ATM) Project
NASA, Goddard Space Flight Center, Wallops Flight Facility
Principal Investigator: Bill Krabill (William.B.Krabill@nasa.gov)

Description of ATM QFIT Output Data
(revised 2009-feb-13 sm)

ATM data is generally distributed in the output format of the
processing  program, qfit, which combines airborne laser ranging
data and aircraft attitude from the INS with positioning
information from a processed kinematic  differential GPS
trajectory.  Qfit output files, which usually have names ending
in a .qi extension, are organized as 32-bit (4-byte)
binary  words, equivalent to a C or IDL long integer, which are
scaled to retain the precision of the measurements.  The format
and the scaling factors are presented below.  The qfit program is
run on an Apple PowerPC processor (and formerly Sun/Motorola).
Accordingly, the output is written in a big-endian format and
must be byte-swapped to be read with a PC (Intel processor)
which uses a little-endian format to store 32-bit integers.

The files are organized into fixed-length logical records. The
beginning of the file contains a header of one or more records
followed by a data segment, in which there is one record per
laser shot. It is not necessary to interpret the header
to use the laser data->

The first word of the header (and the file) is a 32-bit  binary
integer giving the number of bytes in each logical record.  Commonly
qfit files have 12 words per record and this integer will be the
number 48.  The remainder of the initial logical record is padded
with blank bytes (in this case 44 blank bytes). 10-word and 14-word
formats have also been used, as described below.

The remainder of the header is generally a series of logical
records  containing the processing history of the file.  In these
logical records, the  initial word contains a 32-bit binary
integer with a value between -9000000 and -9000008.  The
remaining bytes in each header record is filled with a string of
ascii characters containing information on file processing
history.  In this case, the byte offset (as a longword integer)
from the start of file to the start of laser data will be the
second word of the second record of the header.  (Note: The header
records can be removed by eliminating records that begin with a
negative value since the first word of records in the data segment
is always a positive number.)

In the data segment of the file, the information contained in
words 1-11 of the output record pertains to the  laser pulse, its
footprint, and aircraft attitude.  The last word of each record is
always the GPS time of day when the laser measurement was acquired.

Prior to 2008 surveys, the GPS trajectory was edited to restrict PDOP<9
in order to limit GPS errors to be less than roughly 5cm.  The output
survey data would therefore have occasional gaps where the PDOP>9.
Some applications of ATM data have less stringent accuracy requirements
that would be better served by preserving the data in these gaps.
Starting in 2008, the PDOP limit was changed to 20, which could allow
occasional GPS errors up to about 15cm.  The PDOP value is carried in
the qfit output and can be used to edit data for applications requiring
greater precision.   Any file in the 10-word format, or files in the 12-word
format processed prior to January 2009, will have PDOP limited
<9.

The three data formats are described below.  The format is designated by
the logical record length given in the first word of the data file.

The qi 12-word format (in use since 2006):
Word #       Content
   1    Relative Time (msec from start of data file)
   2    Laser Spot Latitude (degrees X 1,000,000)
   3    Laser Spot Longitude (degrees X 1,000,000)
   4    Elevation (millimeters)
   5    Start Pulse Signal Strength (relative)
   6    Reflected Laser Signal Strength (relative)
   7    Scan Azimuth (degrees X 1,000)
   8    Pitch (degrees X 1,000)
   9    Roll (degrees X 1,000)
  10    GPS PDOP (dilution of precision) (X 10)
  11    Laser received pulse width (digitizer samples)
  12    GPS Time packed (example: 153320100 = 15h 33m 20s 100ms)

10-word format (used prior to 2006):
Word #       Content
   1    Relative Time (msec from start of data file)
   2    Laser Spot Latitude (degrees X 1,000,000)
   3    Laser Spot Longitude (degrees X 1,000,000)
   4    Elevation (millimeters)
   5    Start Pulse Signal Strength (relative)
   6    Reflected Laser Signal Strength (relative)
   7    Scan Azimuth (degrees X 1,000)
   8    Pitch (degrees X 1,000)
   9    Roll (degrees X 1,000)
  10    GPS Time packed (example: 153320100 = 15h 33m 20s 100ms)


Between 1997 and 2004 some ATM surveys included
a separate sensor to measure passive brightness.
In the 14-word format, words 10-13 pertain to the
passive brightness signal, which is essentially a relative
measure of radiance reflected from the  earth's surface within
the vicinity of the laser pulse.  The horizontal position of the
passive footprint is determined relative to the laser footprint
by a  delay formulated during ground testing at Wallops.  The
elevation of the footprint is synthesized from surrounding laser
elevation data->  NOTE:  The passive data is not calibrated and
its use, if any, should  be qualitative in nature.  It may aid
the interpretation of terrain features. The measurement capability
was engineered into the ATM sensors to aid in the identification
of the water/beach interface acquired with  the instrument in
coastal mapping applications.

14-word format:
Word #       Content
   1    Relative Time (msec from start of data file)
   2    Laser Spot Latitude (degrees X 1,000,000)
   3    Laser Spot Longitude (degrees X 1,000,000)
   4    Elevation (millimeters)
   5    Start Pulse Signal Strength (relative)
   6    Reflected Laser Signal Strength (relative)
   7    Scan Azimuth (degrees X 1,000)
   8    Pitch (degrees X 1,000)
   9    Roll (degrees X 1,000)
  10    Passive Signal (relative)
  11    Passive Footprint Latitude (degrees X 1,000,000)
  12    Passive Footprint Longitude (degrees X 1,000,000)
  13    Passive Footprint Synthesized Elevation (millimeters)
  14    GPS Time packed (example: 153320100 = 15h 33m 20s 100ms)
*/

#include "QfitReader.hpp"

#include <pdal/PointView.hpp>
#include <pdal/util/ProgramArgs.hpp>

#pragma warning(disable : 4127) // conditional expression is constant

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.qfit",
    "QFIT Reader",
    "https://pdal.org/stages/readers.qfit.html",
    {"qi"}};

CREATE_STATIC_STAGE(QfitReader, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void addOption(pdal_options_t* options, const std::string& key, double value)
{
    pdal_options_add_f64(options, key.c_str(), value);
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

std::string QfitReader::getName() const
{
    return s_info.name;
}

QfitReader::QfitReader() : pdal::Reader() {}

QfitReader::~QfitReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

void QfitReader::initialize()
{
    m_rustIndex = 0;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);
    addOption(options, "flip_coordinates", m_flip_x ? "true" : "false");
    addOption(options, "scale_z", m_scale_z);

    pdal_reader_t* reader = pdal_reader_create_qfit(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust QFIT reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust QFIT reader failed.");
}

void QfitReader::addArgs(ProgramArgs& args)
{
    args.add("flip_coordinates", "Flip coordinates from 0-360 to -180-180",
             m_flip_x);
    args.add("scale_z", "Z scale. Use 0.001 to go from mm to m", m_scale_z,
             0.001);
}

void QfitReader::addDimensions(PointLayoutPtr layout)
{
    m_dims.clear();
    uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        char* rawName = pdal_point_view_dim_name(m_rustView, idx);
        if (!rawName)
            continue;
        std::string name(rawName);
        pdal_string_free(rawName);

        Dimension::Id id =
            layout->registerOrAssignDim(name, Dimension::Type::Double);
        m_dims.push_back(id);
    }
}

void QfitReader::ready(PointTableRef)
{
    m_rustIndex = 0;
}

point_count_t QfitReader::read(PointViewPtr data, point_count_t count)
{
    PointId nextId = data->size();
    point_count_t numRead = 0;
    while (numRead < count && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        copyPoint(data, nextId);

        if (m_cb)
            m_cb(*data, nextId);

        numRead++;
        nextId++;
        m_rustIndex++;
    }

    return numRead;
}

void QfitReader::copyPoint(PointViewPtr data, PointId outIdx)
{
    for (Dimension::Id dim : m_dims)
    {
        data->setField(dim, outIdx,
                       pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                               Dimension::name(dim).c_str()));
    }
}

void QfitReader::done(PointTableRef) {}

} // namespace pdal
