use defmt::trace;
#[allow(unused_imports)]
use embassy_stm32::flash::{Async, Blocking, Flash};
use super::Error;

pub struct ChunkedSector<
    const SECTOR_OFFSET: usize,
    const SECTOR_SIZE: usize,
    const CHUNK_SIZE: usize,
    MODE
> {
    flash: Flash<'static, MODE>,
}

#[cfg(not(feature = "stm32f405rg"))]
impl<const SECTOR_OFFSET: usize, const SECTOR_SIZE: usize, const CHUNK_SIZE: usize, MODE>
    ChunkedSector<SECTOR_OFFSET, SECTOR_SIZE, CHUNK_SIZE, MODE>
{
    fn find_last_chunk_pos(buffer: &[u8]) -> Option<usize> {
        match Self::find_first_empty_chunk_pos(&buffer) {
            Some(pos) => {
                if pos == SECTOR_OFFSET {
                    return None;
                }
                Some(pos - CHUNK_SIZE)
            }
            None => Some(SECTOR_OFFSET + SECTOR_SIZE - CHUNK_SIZE),
        }
    }

    fn find_first_empty_chunk_pos(buffer: &[u8]) -> Option<usize> {
        for (index, chunk) in buffer.chunks_exact(CHUNK_SIZE).enumerate() {
            if chunk.iter().all(|&byte| byte == 0xff) {
                return Some(SECTOR_OFFSET + index * CHUNK_SIZE);
            }
        }
        None
    }

    pub fn new_blocking(flash: Flash<'static, MODE>) -> Self {
        Self { flash }
    }

    // read last not empty chunk
    pub fn blocking_read(&mut self, chunk: &mut Chunk<CHUNK_SIZE>) -> Result<(), Error> {
        let mut read_data: [u8; SECTOR_SIZE] = [0x0; SECTOR_SIZE];
        self.flash
            .blocking_read(SECTOR_OFFSET as u32, &mut read_data)
            .map_err(|e| Error::Flash(e))?;
        let last_chunk_pos = Self::find_last_chunk_pos(&read_data).ok_or(Error::NoData)?;
        trace!("Last_chunk_pos: {}", last_chunk_pos);
        chunk.data
            .copy_from_slice(&read_data[last_chunk_pos - SECTOR_OFFSET..last_chunk_pos - SECTOR_OFFSET + CHUNK_SIZE]);
        trace!("Load data: {}", chunk.data);
        Ok(())
    }

    // write to first empty chunk
    pub fn blocking_write(&mut self, chunk: &Chunk<CHUNK_SIZE>) -> Result<(), Error> {
        let mut read_data: [u8; SECTOR_SIZE] = [0x0; SECTOR_SIZE];
        self.flash
            .blocking_read(SECTOR_OFFSET as u32, &mut read_data)
            .map_err(|e| Error::Flash(e))?;
        let last_free_cell_pos =
            Self::find_first_empty_chunk_pos(&read_data).unwrap_or_else(|| {
                self.blocking_erase();
                SECTOR_OFFSET
            });
        trace!("Last free pos: {}", last_free_cell_pos);
        self.flash
            .blocking_write((last_free_cell_pos) as u32, &chunk.data)
            .unwrap();
        Ok(())
    }

    // erase sector
    pub fn blocking_erase(&mut self) {
        self.flash
        .blocking_erase(
            SECTOR_OFFSET as u32,
            (SECTOR_OFFSET + SECTOR_SIZE - 1) as u32,
        )
        .unwrap();
    }
}

#[cfg(feature = "stm32f405rg")]
impl<const SECTOR_OFFSET: usize, const SECTOR_SIZE: usize, const CHUNK_SIZE: usize>
    ChunkedSector<SECTOR_OFFSET, SECTOR_SIZE, CHUNK_SIZE, Async>
{
    pub fn new(flash: Flash<'static, Async>) -> Self {
        Self { flash }
    }

    #[inline]
    const fn chunks_len() -> usize {
        SECTOR_SIZE/CHUNK_SIZE
    }

    #[inline]
    const fn last_chunk_pos() -> usize {
        SECTOR_OFFSET + SECTOR_SIZE - CHUNK_SIZE
    }

    fn find_first_empty_chunk_pos(&mut self) -> Result<Option<usize>, Error> {
        for i in 0..Self::chunks_len() {
            let mut read_data: [u8; CHUNK_SIZE] = [0x0; CHUNK_SIZE];
            let chunk_pos = SECTOR_OFFSET + (i*CHUNK_SIZE);
            self.flash
                .blocking_read(chunk_pos as u32, &mut read_data)
                .map_err(|e| Error::Flash(e))?;
            if Chunk::<CHUNK_SIZE>::slice_is_empty(&read_data) {
                return Ok(Some(chunk_pos));
            }
        }
        return Ok(None)
    }

    fn find_last_chunk_pos(&mut self) -> Result<usize, Error> {
        match self.find_first_empty_chunk_pos()? {
            Some(addr) => {
                if addr == SECTOR_OFFSET {
                    return Err(Error::NoData);
                } else {
                    return Ok(addr - CHUNK_SIZE);
                }
            },
            None => return Ok(Self::last_chunk_pos()),
        }
    }

    // read last not empty chunk
    pub fn blocking_read(&mut self, chunk: &mut Chunk<CHUNK_SIZE>) -> Result<(), Error> {
        let last_chunk_pos = self.find_last_chunk_pos()?;
        trace!("Last_chunk_pos: {}", last_chunk_pos);
        self.flash
            .blocking_read(last_chunk_pos as u32, &mut chunk.data)
            .map_err(|e| Error::Flash(e))?;
        trace!("Load data: {}", chunk.data);
        Ok(())
    }

    // write to first empty chunk
    pub fn blocking_write(&mut self, chunk: &Chunk<CHUNK_SIZE>) -> Result<(), Error> {
        let first_free_chunk_pos: usize;
        if let Some(chunk_pos) = self.find_first_empty_chunk_pos()? {
            first_free_chunk_pos = chunk_pos;
        } else {
            self.blocking_erase();
            first_free_chunk_pos = SECTOR_OFFSET;
        }
        trace!("First free pos: {}", first_free_chunk_pos);
        self.flash
            .blocking_write((first_free_chunk_pos) as u32, &chunk.data)
            .unwrap();
        Ok(())
    }

    // erase sector
    pub fn blocking_erase(&mut self) {
        self.flash
        .blocking_erase(
            SECTOR_OFFSET as u32,
            (SECTOR_OFFSET + SECTOR_SIZE - 1) as u32,
        )
        .unwrap();
    }

    // write to first empty chunk
    pub async fn write(&mut self, chunk: &Chunk<CHUNK_SIZE>) -> Result<(), Error> {
        let first_free_chunk_pos: usize;
        if let Some(chunk_pos) = self.find_first_empty_chunk_pos()? {
            first_free_chunk_pos = chunk_pos;
        } else {
            self.erase().await;
            first_free_chunk_pos = SECTOR_OFFSET;
        }
        trace!("First free pos: {}", first_free_chunk_pos);
        self.flash
            .write((first_free_chunk_pos) as u32, &chunk.data).await
            .unwrap();
        Ok(())
    }

    // erase sector
    pub async fn erase(&mut self) {
        self.flash
        .erase(
            SECTOR_OFFSET as u32,
            (SECTOR_OFFSET + SECTOR_SIZE - 1) as u32,
        ).await
        .unwrap();
    }
}

pub struct Chunk<const C: usize> {
    pub data: [u8; C],
}

impl<const C: usize> Chunk<C> {
    #[inline]
    pub fn new() -> Self {
        Self { data: [0; C] }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        Self::slice_is_empty(&self.data)
    }

    #[inline]
    pub fn slice_is_empty(data: &[u8]) -> bool {
        data.iter().all(|&byte| byte == 0xff)
    }

    #[inline]
    pub fn copy_from_slice(&mut self, data: &[u8]) {
        self.data.copy_from_slice(data);
    }
}

impl<const C: usize, const N: usize> From<Chunk<C>> for [u32; N] {
    fn from(chunk: Chunk<C>) -> Self {
        let mut u32_array = [0u32; N];
        for (i, chunk) in chunk.data.chunks_exact(size_of::<u32>()).enumerate() {
            u32_array[i] = u32::from_be_bytes(chunk.try_into().unwrap());
        }
        u32_array
    }
}

impl<const C: usize, const N: usize> From<[u32; N]> for Chunk<C> {
    fn from(data: [u32; N]) -> Self {
        let mut bytes = [0u8; C];
        for (i, &num) in data.iter().enumerate() {
            let chunk = num.to_be_bytes(); // Convert each u32 to a 4-byte array (big-endian)
            bytes[i * size_of::<u32>()..i * size_of::<u32>() + size_of::<u32>()]
                .copy_from_slice(&chunk);
        }
        Chunk { data: bytes }
    }
}

impl<const C: usize, const N: usize> From<Chunk<C>> for [f32; N] {
    fn from(chunk: Chunk<C>) -> Self {
        let mut f32_array = [0.0; N];
        for (i, chunk) in chunk.data.chunks_exact(size_of::<f32>()).enumerate() {
            f32_array[i] = f32::from_be_bytes(chunk.try_into().unwrap());
        }
        f32_array
    }
}

impl<const C: usize, const N: usize> From<[f32; N]> for Chunk<C> {
    fn from(data: [f32; N]) -> Self {
        let mut bytes = [0u8; C];
        for (i, &num) in data.iter().enumerate() {
            let chunk = num.to_be_bytes(); // Convert each u32 to a 4-byte array (big-endian)
            bytes[i * size_of::<f32>()..i * size_of::<f32>() + size_of::<f32>()]
                .copy_from_slice(&chunk);
        }
        Chunk { data: bytes }
    }
}