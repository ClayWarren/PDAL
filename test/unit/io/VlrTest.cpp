/******************************************************************************
 * Copyright (c) 2026, Hobu Inc. (info@hobu.co)
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in the
 *       documentation and/or other materials provided with the distribution.
 *     * Neither the name of Hobu, Inc. nor the names of its contributors may
 *       be used to endorse or promote products derived from this software
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 ****************************************************************************/

#include <limits>
#include <string>
#include <vector>

#include <pdal/pdal_test_main.hpp>
#include <pdal/util/Inserter.hpp>

#include <io/private/las/Vlr.hpp>

namespace pdal
{

namespace
{

const int EvlrHeaderSize = 60;

std::vector<char> evlrHeader(const std::string& userId, uint16_t recordId,
    uint64_t dataSize)
{
    std::vector<char> header(EvlrHeaderSize);
    LeInserter out(header.data(), header.size());
    out << uint16_t(0);
    out.put(userId, 16);
    out << recordId << dataSize;
    out.put(std::string(), 32);
    return header;
}

} // unnamed namespace

TEST(VlrCatalogTest, evlrDescriptionLengthWrap)
{
    const std::string userId("overflow-test");
    const uint16_t recordId = 42;
    std::vector<char> header = evlrHeader(userId, recordId,
        (std::numeric_limits<uint32_t>::max)());

    int fetchCount = 0;
    uint64_t headerOffset = 0;
    int32_t headerSize = 0;
    las::VlrCatalog catalog(
        [&header, &fetchCount, &headerOffset, &headerSize]
        (uint64_t offset, int32_t size)
        {
            ++fetchCount;
            if (fetchCount > 1)
                throw pdal_error("Unexpected EVLR payload fetch.");
            headerOffset = offset;
            headerSize = size;
            return header;
        });

    catalog.load(0, 0, 100, 1);
    EXPECT_EQ(fetchCount, 1);
    EXPECT_EQ(headerOffset, 100u);
    EXPECT_EQ(headerSize, EvlrHeaderSize);

    std::string description;
    std::vector<char> data;
    EXPECT_NO_THROW(data = catalog.fetchWithDescription(userId, recordId,
        description));
    EXPECT_TRUE(data.empty());
    EXPECT_EQ(fetchCount, 1);
}

TEST(VlrCatalogTest, invalidDescriptionOffset)
{
    const std::string userId("offset-test");
    const uint16_t recordId = 43;
    std::vector<char> header = evlrHeader(userId, recordId, 1);
    int fetchCount = 0;
    las::VlrCatalog catalog([&header, &fetchCount](uint64_t, int32_t)
        {
            ++fetchCount;
            if (fetchCount > 1)
                throw pdal_error("Unexpected EVLR payload fetch.");
            return header;
        });

    // The test callback intentionally accepts an otherwise impossible header
    // offset so that adding the EVLR header size wraps the catalog data offset.
    const uint64_t evlrOffset =
        (std::numeric_limits<uint64_t>::max)() - 39;
    catalog.load(0, 0, evlrOffset, 1);

    std::string description;
    EXPECT_THROW(catalog.fetchWithDescription(userId, recordId, description),
        pdal_error);
    EXPECT_EQ(fetchCount, 1);
}

} // namespace pdal
